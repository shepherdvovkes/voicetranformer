// Linux платформо-специфичная функциональность
use super::PlatformAudio;

#[cfg(target_os = "linux")]
use alsa::{Direction, PCM};

/// Платформо-специфичная реализация для Linux с ALSA
pub struct AlsaPlatform {
    sample_rate: f32,
    buffer_size: usize,
    is_running: bool,
    #[cfg(target_os = "linux")]
    pcm: Option<PCM>,
}

#[derive(Debug)]
pub enum AlsaError {
    InitializationFailed(String),
    AlsaError(String),
    DeviceNotFound,
    UnsupportedFormat,
}

impl std::fmt::Display for AlsaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlsaError::InitializationFailed(msg) => write!(f, "Ошибка инициализации: {}", msg),
            AlsaError::AlsaError(msg) => write!(f, "Ошибка ALSA: {}", msg),
            AlsaError::DeviceNotFound => write!(f, "Аудио устройство не найдено"),
            AlsaError::UnsupportedFormat => write!(f, "Неподдерживаемый формат"),
        }
    }
}

impl std::error::Error for AlsaError {}

impl PlatformAudio for AlsaPlatform {
    type Error = AlsaError;

    fn initialize() -> Result<Self, Self::Error> {
        println!("🐧 Инициализация ALSA на Linux...");
        
        #[cfg(target_os = "linux")]
        {
            match setup_alsa_pcm() {
                Ok(pcm) => {
                    println!("✅ ALSA успешно инициализирован");
                    Ok(AlsaPlatform {
                        sample_rate: 44100.0,
                        buffer_size: 512,
                        is_running: false,
                        pcm: Some(pcm),
                    })
                }
                Err(e) => Err(AlsaError::InitializationFailed(e.to_string())),
            }
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            Err(AlsaError::InitializationFailed(
                "Не компилируется для Linux".to_string(),
            ))
        }
    }

    fn get_sample_rate(&self) -> f32 {
        self.sample_rate
    }

    fn get_buffer_size(&self) -> usize {
        self.buffer_size
    }

    fn start(&mut self) -> Result<(), Self::Error> {
        if self.is_running {
            return Ok(());
        }

        #[cfg(target_os = "linux")]
        {
            if let Some(ref pcm) = self.pcm {
                pcm.prepare().map_err(|e| {
                    AlsaError::AlsaError(format!("Не удалось подготовить PCM: {}", e))
                })?;
            }
        }

        self.is_running = true;
        println!("🎵 ALSA поток запущен");
        Ok(())
    }

    fn stop(&mut self) -> Result<(), Self::Error> {
        if !self.is_running {
            return Ok(());
        }

        #[cfg(target_os = "linux")]
        {
            if let Some(ref pcm) = self.pcm {
                pcm.drop().map_err(|e| {
                    AlsaError::AlsaError(format!("Не удалось остановить PCM: {}", e))
                })?;
            }
        }

        self.is_running = false;
        println!("🛑 ALSA поток остановлен");
        Ok(())
    }

    fn supports_low_latency(&self) -> bool {
        true // ALSA поддерживает низкую задержку с правильной настройкой
    }

    fn platform_info(&self) -> String {
        format!(
            "Linux ALSA (SR: {:.0} Гц, Buffer: {} сэмплов)",
            self.sample_rate, self.buffer_size
        )
    }
}

impl AlsaPlatform {
    /// Устанавливает размер буфера для оптимизации производительности
    pub fn set_buffer_size(&mut self, buffer_size: usize) -> Result<(), AlsaError> {
        self.buffer_size = buffer_size;
        println!("📊 Размер буфера ALSA установлен: {} сэмплов", buffer_size);
        Ok(())
    }
    
    /// Получает список доступных аудио устройств
    pub fn list_devices(&self) -> Vec<String> {
        #[cfg(target_os = "linux")]
        {
            // Заглушка - в реальной реализации здесь будет перечисление устройств ALSA
            vec![
                "default".to_string(),
                "hw:0,0".to_string(),
                "plughw:0,0".to_string(),
            ]
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            vec![]
        }
    }
}

/// Настройка ALSA PCM устройства
#[cfg(target_os = "linux")]
fn setup_alsa_pcm() -> Result<PCM, Box<dyn std::error::Error>> {
    use alsa::pcm::{HwParams, Format, Access};
    
    // Открываем устройство по умолчанию для воспроизведения
    let pcm = PCM::new("default", Direction::Playback, false)?;
    
    // Настройка параметров оборудования
    {
        let hwp = HwParams::any(&pcm)?;
        hwp.set_channels(2)?; // Стерео
        hwp.set_rate(44100, alsa::ValueOr::Nearest)?; // 44.1 кГц
        hwp.set_format(Format::float())?; // 32-bit float
        hwp.set_access(Access::RWInterleaved)?;
        hwp.set_buffer_size_near(1024)?; // Размер буфера
        pcm.hw_params(&hwp)?;
    }
    
    // Настройка программных параметров
    {
        let hwp = pcm.hw_params_current()?;
        let swp = pcm.sw_params_current()?;
        swp.set_start_threshold(hwp.get_buffer_size()? - hwp.get_period_size()?)?;
        pcm.sw_params(&swp)?;
    }
    
    Ok(pcm)
}

/// Linux-специфичные утилиты для аудио
pub mod linux_audio_utils {
    
    /// Проверяет доступность различных аудио систем Linux
    pub fn check_audio_systems() -> Vec<String> {
        let mut systems = Vec::new();
        
        // Проверяем ALSA
        #[cfg(target_os = "linux")]
        {
            if std::path::Path::new("/proc/asound").exists() {
                systems.push("ALSA".to_string());
            }
        }
        
        // Проверяем PulseAudio
        if std::process::Command::new("pulseaudio")
            .arg("--check")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
        {
            systems.push("PulseAudio".to_string());
        }
        
        // Проверяем JACK
        if std::process::Command::new("jack_control")
            .arg("status")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
        {
            systems.push("JACK".to_string());
        }
        
        // Проверяем PipeWire
        if std::process::Command::new("pipewire")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
        {
            systems.push("PipeWire".to_string());
        }
        
        systems
    }
    
    /// Получает информацию о системе
    pub fn get_system_info() -> String {
        let audio_systems = check_audio_systems();
        format!(
            "Linux аудио системы: {}",
            if audio_systems.is_empty() {
                "Не обнаружены".to_string()
            } else {
                audio_systems.join(", ")
            }
        )
    }
    
    /// Рекомендации по оптимизации для Linux
    pub fn optimization_tips() -> Vec<String> {
        vec![
            "Используйте PREEMPT_RT ядро для минимальной задержки".to_string(),
            "Настройте приоритеты процессов с помощью rtprio".to_string(),
            "Отключите CPU frequency scaling для аудио приложений".to_string(),
            "Используйте JACK для профессиональной аудио обработки".to_string(),
            "Настройте udev правила для доступа к аудио устройствам".to_string(),
        ]
    }
}