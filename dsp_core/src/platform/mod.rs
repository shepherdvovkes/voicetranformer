// Платформо-специфичная функциональность

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "windows")]
pub mod windows;

// Общий трейт для платформо-специфичной аудио обработки
pub trait PlatformAudio {
    type Error;
    
    /// Инициализирует платформо-специфичную аудио подсистему
    fn initialize() -> Result<Self, Self::Error>
    where
        Self: Sized;
    
    /// Получает доступную частоту дискретизации
    fn get_sample_rate(&self) -> f32;
    
    /// Получает размер буфера
    fn get_buffer_size(&self) -> usize;
    
    /// Запускает аудио поток
    fn start(&mut self) -> Result<(), Self::Error>;
    
    /// Останавливает аудио поток
    fn stop(&mut self) -> Result<(), Self::Error>;
    
    /// Проверяет поддержку низкой задержки
    fn supports_low_latency(&self) -> bool;
    
    /// Получает информацию о платформе
    fn platform_info(&self) -> String;
    
    /// Проверяет поддержку Neural Engine/NPU
    fn supports_neural_engine(&self) -> bool {
        false // По умолчанию не поддерживается
    }
}

// Определяем тип для текущей платформы
#[cfg(target_os = "macos")]
pub type PlatformAudioImpl = macos::CoreAudioPlatform;

#[cfg(target_os = "linux")]
pub type PlatformAudioImpl = linux::AlsaPlatform;

#[cfg(target_os = "windows")]
pub type PlatformAudioImpl = windows::WasapiPlatform;

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
pub type PlatformAudioImpl = DefaultPlatform;

// Заглушка для неподдерживаемых платформ
#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
pub struct DefaultPlatform;

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
impl PlatformAudio for DefaultPlatform {
    type Error = &'static str;
    
    fn initialize() -> Result<Self, Self::Error> {
        Err("Платформа не поддерживается")
    }
    
    fn get_sample_rate(&self) -> f32 { 44100.0 }
    fn get_buffer_size(&self) -> usize { 512 }
    fn start(&mut self) -> Result<(), Self::Error> { Ok(()) }
    fn stop(&mut self) -> Result<(), Self::Error> { Ok(()) }
    fn supports_low_latency(&self) -> bool { false }
    fn platform_info(&self) -> String { "Неизвестная платформа".to_string() }
}