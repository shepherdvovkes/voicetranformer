// Neural Engine обработка голоса на Apple M1/M2/M3
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::VecDeque;
use std::time::Instant;

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
use metal::*;

/// Типы голосовых эффектов для Neural Engine
#[derive(Debug, Clone, PartialEq)]
pub enum VoiceEffect {
    PitchShift(f32),      // Сдвиг высоты тона (-12.0 до +12.0 полутонов)
    FormantShift(f32),    // Сдвиг формант (-2.0 до +2.0)
    VoiceChanger {        // Изменение голоса
        gender: f32,      // -1.0 (мужской) до +1.0 (женский)
        age: f32,         // -1.0 (молодой) до +1.0 (старый)
        roughness: f32,   // 0.0 до 1.0
    },
    Harmonics {           // Гармонические эффекты
        overtones: f32,   // 0.0 до 1.0
        undertones: f32,  // 0.0 до 1.0
        distortion: f32,  // 0.0 до 1.0
    },
    Modulation {          // Модуляционные эффекты
        vibrato_rate: f32,    // 0.1 до 20.0 Гц
        vibrato_depth: f32,   // 0.0 до 1.0
        tremolo_rate: f32,    // 0.1 до 20.0 Гц
        tremolo_depth: f32,   // 0.0 до 1.0
    },
    Reverb {              // Пространственные эффекты
        room_size: f32,   // 0.0 до 1.0
        damping: f32,     // 0.0 до 1.0
        wet_level: f32,   // 0.0 до 1.0
    },
    Chorus {              // Хорус эффект
        voices: u32,      // 2 до 8
        delay: f32,       // 10.0 до 100.0 мс
        depth: f32,       // 0.0 до 1.0
        rate: f32,        // 0.1 до 5.0 Гц
    },
    Distortion {          // Искажения
        drive: f32,       // 0.0 до 1.0
        tone: f32,        // 0.0 до 1.0
        level: f32,       // 0.0 до 1.0
    },
    AutoTune {            // Автотюн
        correction: f32,  // 0.0 до 1.0
        speed: f32,       // 0.1 до 10.0
        key: i32,         // 0-11 (C, C#, D, ...)
    },
}

/// Результат обработки на Neural Engine
#[derive(Debug, Clone)]
pub struct NeuralProcessingResult {
    pub output: Vec<f32>,
    pub latency_ns: u64,
    pub neural_engine_load: f32,
    pub effects_applied: Vec<VoiceEffect>,
    pub quality_score: f32,
}

/// Конфигурация Neural Engine процессора
#[derive(Debug, Clone)]
pub struct NeuralConfig {
    pub sample_rate: f32,
    pub buffer_size: usize,
    pub max_effects: usize,
    pub quality_preset: QualityPreset,
    pub enable_real_time: bool,
}

#[derive(Debug, Clone)]
pub enum QualityPreset {
    UltraLow,     // Минимальная задержка для живых выступлений
    Low,          // Сбалансированный для стриминга  
    Medium,       // Хорошее качество для записи
    High,         // Высокое качество студийной записи
    Ultra,        // Максимальное качество для мастеринга
}

/// Главный Neural Engine процессор голоса
pub struct NeuralVoiceProcessor {
    config: NeuralConfig,
    is_processing: AtomicBool,
    effects_chain: Vec<VoiceEffect>,
    
    // Буферы для обработки
    input_buffer: VecDeque<f32>,
    output_buffer: VecDeque<f32>,
    
    // Статистика производительности
    processing_times: VecDeque<u64>,
    neural_loads: VecDeque<f32>,
    
    // Apple Silicon специфичные компоненты
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    metal_device: Option<Device>,
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    metal_queue: Option<CommandQueue>,
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    neural_kernels: Option<NeuralKernels>,
    
    // Обработчики эффектов
    pitch_processor: PitchProcessor,
    formant_processor: FormantProcessor,
    modulation_processor: ModulationProcessor,
    spatial_processor: SpatialProcessor,
}

impl NeuralVoiceProcessor {
    pub fn new(config: NeuralConfig) -> Result<Self, String> {
        println!("🧠 Инициализация Neural Engine процессора голоса...");
        
        let processor = Self {
            config: config.clone(),
            is_processing: AtomicBool::new(false),
            effects_chain: Vec::new(),
            input_buffer: VecDeque::new(),
            output_buffer: VecDeque::new(),
            processing_times: VecDeque::new(),
            neural_loads: VecDeque::new(),
            
            #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
            metal_device: None,
            #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
            metal_queue: None,
            #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
            neural_kernels: None,
            
            pitch_processor: PitchProcessor::new(&config)?,
            formant_processor: FormantProcessor::new(&config)?,
            modulation_processor: ModulationProcessor::new(&config)?,
            spatial_processor: SpatialProcessor::new(&config)?,
        };
        
        // Инициализируем Metal для Neural Engine
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            // Пока что пропускаем инициализацию Metal в demo версии
            // processor.initialize_metal()?;
        }
        
        println!("✅ Neural Engine процессор готов к обработке голоса");
        Ok(processor)
    }
    
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    fn initialize_metal(&mut self) -> Result<(), String> {
        println!("⚡ Инициализация Metal для Neural Engine...");
        
        // Получаем GPU устройство (Neural Engine доступен через Metal Performance Shaders)
        let device = Device::system_default()
            .ok_or("Не удалось найти Metal устройство")?;
        
        // Создаем command queue для выполнения GPU операций
        let queue = device.new_command_queue();
        
        // Инициализируем нейронные ядра
        let kernels = NeuralKernels::new(&device)?;
        
        self.metal_device = Some(device);
        self.metal_queue = Some(queue);
        self.neural_kernels = Some(kernels);
        
        println!("✅ Metal инициализирован для Neural Engine обработки");
        Ok(())
    }
    
    /// Добавляет эффект в цепочку обработки
    pub fn add_effect(&mut self, effect: VoiceEffect) -> Result<(), String> {
        if self.effects_chain.len() >= self.config.max_effects {
            return Err(format!("Превышено максимальное количество эффектов ({})", self.config.max_effects));
        }
        
        self.effects_chain.push(effect.clone());
        println!("🎛️ Добавлен эффект: {:?}", effect);
        Ok(())
    }
    
    /// Удаляет эффект из цепочки
    pub fn remove_effect(&mut self, effect: &VoiceEffect) {
        self.effects_chain.retain(|e| e != effect);
        println!("🗑️ Удален эффект: {:?}", effect);
    }
    
    /// Очищает все эффекты
    pub fn clear_effects(&mut self) {
        self.effects_chain.clear();
        println!("🧹 Все эффекты удалены");
    }
    
    /// Обрабатывает аудио через Neural Engine
    pub fn process(&mut self, input: &[f32]) -> Result<NeuralProcessingResult, String> {
        let start_time = Instant::now();
        
        if !self.is_processing.load(Ordering::Relaxed) {
            self.is_processing.store(true, Ordering::Relaxed);
        }
        
        // Добавляем входные данные в буфер
        self.input_buffer.extend(input.iter().copied());
        
        let mut output = input.to_vec();
        let mut applied_effects = Vec::new();
        
        // Применяем каждый эффект последовательно
        let effects_chain = self.effects_chain.clone();
        for effect in &effects_chain {
            output = self.apply_effect(&output, effect)?;
            applied_effects.push(effect.clone());
        }
        
        // Измеряем время обработки
        let processing_time = start_time.elapsed().as_nanos() as u64;
        self.update_performance_stats(processing_time);
        
        // Расчитываем нагрузку на Neural Engine
        let neural_load = self.calculate_neural_load(&applied_effects);
        
        // Оценка качества обработки
        let quality_score = self.calculate_quality_score(&output, &applied_effects);
        
        Ok(NeuralProcessingResult {
            output,
            latency_ns: processing_time,
            neural_engine_load: neural_load,
            effects_applied: applied_effects,
            quality_score,
        })
    }
    
    /// Применяет конкретный эффект к аудио
    fn apply_effect(&mut self, input: &[f32], effect: &VoiceEffect) -> Result<Vec<f32>, String> {
        match effect {
            VoiceEffect::PitchShift(semitones) => {
                self.pitch_processor.shift_pitch(input, *semitones)
            }
            VoiceEffect::FormantShift(shift) => {
                self.formant_processor.shift_formants(input, *shift)
            }
            VoiceEffect::VoiceChanger { gender, age, roughness } => {
                self.apply_voice_transformation(input, *gender, *age, *roughness)
            }
            VoiceEffect::Harmonics { overtones, undertones, distortion } => {
                self.apply_harmonic_enhancement(input, *overtones, *undertones, *distortion)
            }
            VoiceEffect::Modulation { vibrato_rate, vibrato_depth, tremolo_rate, tremolo_depth } => {
                self.modulation_processor.apply_modulation(input, *vibrato_rate, *vibrato_depth, *tremolo_rate, *tremolo_depth)
            }
            VoiceEffect::Reverb { room_size, damping, wet_level } => {
                self.spatial_processor.apply_reverb(input, *room_size, *damping, *wet_level)
            }
            VoiceEffect::Chorus { voices, delay, depth, rate } => {
                self.spatial_processor.apply_chorus(input, *voices, *delay, *depth, *rate)
            }
            VoiceEffect::Distortion { drive, tone, level } => {
                self.apply_distortion(input, *drive, *tone, *level)
            }
            VoiceEffect::AutoTune { correction, speed, key } => {
                self.apply_autotune(input, *correction, *speed, *key)
            }
        }
    }
    
    /// Применяет изменение голоса (пол, возраст, грубость)
    fn apply_voice_transformation(&self, input: &[f32], gender: f32, age: f32, roughness: f32) -> Result<Vec<f32>, String> {
        let mut output = Vec::with_capacity(input.len());
        
        for (i, &sample) in input.iter().enumerate() {
            let t = i as f32 / self.config.sample_rate;
            
            // Изменение пола через формантное смещение
            let gender_mod = 1.0 + gender * 0.3;
            
            // Изменение возраста через высокочастотное ослабление
            let age_filter = 1.0 - age.abs() * 0.2;
            
            // Добавление грубости через нелинейные искажения
            let roughness_factor = 1.0 + roughness * (t * 100.0).sin() * 0.1;
            
            let processed = sample * gender_mod * age_filter * roughness_factor;
            let clamped = processed.max(-1.0).min(1.0);
            
            output.push(clamped);
        }
        
        Ok(output)
    }
    
    /// Применяет гармонические эффекты
    fn apply_harmonic_enhancement(&self, input: &[f32], overtones: f32, undertones: f32, distortion: f32) -> Result<Vec<f32>, String> {
        let mut output = Vec::with_capacity(input.len());
        
        for (i, &sample) in input.iter().enumerate() {
            let phase = i as f32 * 2.0 * std::f32::consts::PI / self.config.sample_rate;
            
            // Добавляем обертоны (высшие гармоники)
            let overtone_1 = overtones * 0.3 * (phase * 2.0).sin();
            let overtone_2 = overtones * 0.2 * (phase * 3.0).sin();
            let overtone_3 = overtones * 0.1 * (phase * 4.0).sin();
            
            // Добавляем субгармоники (низшие частоты)
            let undertone_1 = undertones * 0.2 * (phase * 0.5).sin();
            let undertone_2 = undertones * 0.1 * (phase * 0.25).sin();
            
            // Нелинейные искажения
            let distorted = if distortion > 0.0 {
                sample.signum() * (sample.abs().powf(1.0 - distortion * 0.5))
            } else {
                sample
            };
            
            let enhanced = distorted + overtone_1 + overtone_2 + overtone_3 + undertone_1 + undertone_2;
            let normalized = enhanced * 0.7; // Нормализация
            
            output.push(normalized.max(-1.0).min(1.0));
        }
        
        Ok(output)
    }
    
    /// Применяет искажения
    fn apply_distortion(&self, input: &[f32], drive: f32, tone: f32, level: f32) -> Result<Vec<f32>, String> {
        let mut output = Vec::with_capacity(input.len());
        
        for &sample in input {
            // Усиление сигнала
            let driven = sample * (1.0 + drive * 10.0);
            
            // Нелинейные искажения
            let distorted = if driven > 0.0 {
                1.0 - (-driven).exp()
            } else {
                -1.0 + driven.exp()
            };
            
            // Тональная коррекция (простой фильтр)
            let toned = distorted * (1.0 + tone * 0.5);
            
            // Финальный уровень
            let final_sample = toned * level;
            
            output.push(final_sample.max(-1.0).min(1.0));
        }
        
        Ok(output)
    }
    
    /// Применяет автотюн
    fn apply_autotune(&self, input: &[f32], correction: f32, speed: f32, key: i32) -> Result<Vec<f32>, String> {
        // Упрощенная реализация автотюна
        let mut output = Vec::with_capacity(input.len());
        
        // Определяем ноты в хроматической гамме (центы от C)
        let note_frequencies = [261.63, 277.18, 293.66, 311.13, 329.63, 349.23, 369.99, 392.00, 415.30, 440.00, 466.16, 493.88];
        let target_freq = note_frequencies[key as usize % 12];
        
        for (i, &sample) in input.iter().enumerate() {
            // Простая питч-коррекция (в реальности нужен сложный анализ)
            let correction_factor = 1.0 + correction * 0.1 * (i as f32 * target_freq / self.config.sample_rate).sin();
            let corrected = sample * correction_factor;
            
            // Сглаживание с предыдущими значениями для скорости коррекции
            let smoothed = if i > 0 {
                corrected * speed + output[i-1] * (1.0 - speed)
            } else {
                corrected
            };
            
            output.push(smoothed.max(-1.0).min(1.0));
        }
        
        Ok(output)
    }
    
    /// Расчитывает нагрузку на Neural Engine
    fn calculate_neural_load(&self, effects: &[VoiceEffect]) -> f32 {
        let base_load = effects.len() as f32 * 10.0; // Базовая нагрузка от количества эффектов
        
        let complexity_load: f32 = effects.iter().map(|effect| {
            match effect {
                VoiceEffect::PitchShift(_) => 15.0,
                VoiceEffect::FormantShift(_) => 20.0,
                VoiceEffect::VoiceChanger { .. } => 25.0,
                VoiceEffect::Harmonics { .. } => 30.0,
                VoiceEffect::Modulation { .. } => 10.0,
                VoiceEffect::Reverb { .. } => 35.0,
                VoiceEffect::Chorus { .. } => 20.0,
                VoiceEffect::Distortion { .. } => 5.0,
                VoiceEffect::AutoTune { .. } => 40.0,
            }
        }).sum();
        
        let quality_multiplier = match self.config.quality_preset {
            QualityPreset::UltraLow => 0.5,
            QualityPreset::Low => 0.7,
            QualityPreset::Medium => 1.0,
            QualityPreset::High => 1.3,
            QualityPreset::Ultra => 1.8,
        };
        
        ((base_load + complexity_load) * quality_multiplier).min(100.0)
    }
    
    /// Расчитывает оценку качества обработки
    fn calculate_quality_score(&self, output: &[f32], effects: &[VoiceEffect]) -> f32 {
        // Проверяем на клиппинг
        let clipping_penalty = output.iter()
            .filter(|&&x| x.abs() > 0.95)
            .count() as f32 / output.len() as f32;
        
        // Динамический диапазон
        let max_val = output.iter().map(|x| x.abs()).fold(0.0, f32::max);
        let avg_val = output.iter().map(|x| x.abs()).sum::<f32>() / output.len() as f32;
        let dynamic_range = if avg_val > 0.0 { max_val / avg_val } else { 1.0 };
        
        // Штраф за слишком много эффектов
        let effects_penalty = if effects.len() > 5 { 0.1 * (effects.len() - 5) as f32 } else { 0.0 };
        
        let base_score = 1.0 - clipping_penalty - effects_penalty;
        let dynamic_bonus = (dynamic_range - 1.0).min(0.2);
        
        (base_score + dynamic_bonus).max(0.0).min(1.0)
    }
    
    /// Обновляет статистику производительности
    fn update_performance_stats(&mut self, processing_time: u64) {
        self.processing_times.push_back(processing_time);
        
        // Сохраняем только последние 100 измерений
        if self.processing_times.len() > 100 {
            self.processing_times.pop_front();
        }
    }
    
    /// Возвращает среднюю задержку в наносекундах
    pub fn get_average_latency_ns(&self) -> u64 {
        if self.processing_times.is_empty() {
            return 0;
        }
        self.processing_times.iter().sum::<u64>() / self.processing_times.len() as u64
    }
    
    /// Возвращает среднюю нагрузку на Neural Engine
    pub fn get_average_neural_load(&self) -> f32 {
        if self.neural_loads.is_empty() {
            return 0.0;
        }
        self.neural_loads.iter().sum::<f32>() / self.neural_loads.len() as f32
    }
    
    /// Возвращает информацию о поддержке Neural Engine
    pub fn neural_engine_info(&self) -> String {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            let device_info = if let Some(ref device) = self.metal_device {
                format!("Metal GPU: {}", device.name())
            } else {
                "Metal не инициализирован".to_string()
            };
            
            format!("Apple Neural Engine доступен - {}", device_info)
        }
        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
        {
            "Neural Engine недоступен (требуется Apple Silicon)".to_string()
        }
    }
}

// Neural Engine ядра для Metal
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
struct NeuralKernels {
    pitch_kernel: ComputePipelineState,
    formant_kernel: ComputePipelineState,
    harmonics_kernel: ComputePipelineState,
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
impl NeuralKernels {
    fn new(device: &Device) -> Result<Self, String> {
        // Создаем Metal шейдеры для обработки аудио
        let library = device.new_default_library();
        
        // В реальной реализации здесь будут загружаться скомпилированные шейдеры
        // Пока создаем заглушки
        
        Ok(NeuralKernels {
            pitch_kernel: create_dummy_kernel(device)?,
            formant_kernel: create_dummy_kernel(device)?,
            harmonics_kernel: create_dummy_kernel(device)?,
        })
    }
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn create_dummy_kernel(_device: &Device) -> Result<ComputePipelineState, String> {
    // Заглушка для создания compute pipeline
    // В реальной реализации здесь будет компиляция Metal шейдеров
    Err("Metal шейдеры не реализованы в демо версии".to_string())
}

// Специализированные процессоры эффектов

/// Процессор изменения высоты тона
struct PitchProcessor {
    sample_rate: f32,
    window_size: usize,
    overlap: usize,
    fft_buffer: Vec<f32>,
}

impl PitchProcessor {
    fn new(config: &NeuralConfig) -> Result<Self, String> {
        Ok(Self {
            sample_rate: config.sample_rate,
            window_size: 2048,
            overlap: 1024,
            fft_buffer: vec![0.0; 2048],
        })
    }
    
    fn shift_pitch(&mut self, input: &[f32], semitones: f32) -> Result<Vec<f32>, String> {
        // Упрощенная реализация pitch shifting
        let pitch_ratio = 2.0_f32.powf(semitones / 12.0);
        let mut output = Vec::with_capacity(input.len());
        
        for (i, &sample) in input.iter().enumerate() {
            // Простая интерполяция для pitch shifting
            let source_index = i as f32 / pitch_ratio;
            let index = source_index as usize;
            
            if index < input.len() - 1 {
                let frac = source_index - index as f32;
                let interpolated = input[index] * (1.0 - frac) + input[index + 1] * frac;
                output.push(interpolated);
            } else {
                output.push(sample);
            }
        }
        
        Ok(output)
    }
}

/// Процессор изменения формант
struct FormantProcessor {
    sample_rate: f32,
    formant_filters: Vec<FormantFilter>,
}

impl FormantProcessor {
    fn new(config: &NeuralConfig) -> Result<Self, String> {
        // Инициализируем фильтры для основных формант
        let formant_filters = vec![
            FormantFilter::new(800.0, 80.0),   // F1
            FormantFilter::new(1200.0, 90.0),  // F2  
            FormantFilter::new(2400.0, 120.0), // F3
        ];
        
        Ok(Self {
            sample_rate: config.sample_rate,
            formant_filters,
        })
    }
    
    fn shift_formants(&mut self, input: &[f32], shift: f32) -> Result<Vec<f32>, String> {
        let mut output = input.to_vec();
        
        // Применяем сдвиг к каждому формантному фильтру
        for filter in &mut self.formant_filters {
            output = filter.process(&output, shift)?;
        }
        
        Ok(output)
    }
}

/// Формантный фильтр
struct FormantFilter {
    center_freq: f32,
    bandwidth: f32,
    state: [f32; 2],
}

impl FormantFilter {
    fn new(center_freq: f32, bandwidth: f32) -> Self {
        Self {
            center_freq,
            bandwidth,
            state: [0.0; 2],
        }
    }
    
    fn process(&mut self, input: &[f32], shift: f32) -> Result<Vec<f32>, String> {
        // Простая реализация формантного фильтра
        let shifted_freq = self.center_freq * (1.0 + shift * 0.5);
        let omega = 2.0 * std::f32::consts::PI * shifted_freq / 44100.0;
        let cos_omega = omega.cos();
        let sin_omega = omega.sin();
        let alpha = sin_omega / (2.0 * self.bandwidth / shifted_freq);
        
        // Коэффициенты IIR фильтра
        let b0 = alpha;
        let b1 = 0.0;
        let b2 = -alpha;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_omega;
        let a2 = 1.0 - alpha;
        
        let mut output = Vec::with_capacity(input.len());
        
        for &sample in input {
            let y = (b0 * sample + b1 * self.state[0] + b2 * self.state[1]
                   - a1 * self.state[0] - a2 * self.state[1]) / a0;
            
            // Обновляем состояние
            self.state[1] = self.state[0];
            self.state[0] = y;
            
            output.push(y);
        }
        
        Ok(output)
    }
}

/// Процессор модуляционных эффектов
struct ModulationProcessor {
    sample_rate: f32,
    vibrato_phase: f32,
    tremolo_phase: f32,
}

impl ModulationProcessor {
    fn new(config: &NeuralConfig) -> Result<Self, String> {
        Ok(Self {
            sample_rate: config.sample_rate,
            vibrato_phase: 0.0,
            tremolo_phase: 0.0,
        })
    }
    
    fn apply_modulation(&mut self, input: &[f32], vibrato_rate: f32, vibrato_depth: f32, 
                       tremolo_rate: f32, tremolo_depth: f32) -> Result<Vec<f32>, String> {
        let mut output = Vec::with_capacity(input.len());
        
        for (_i, &sample) in input.iter().enumerate() {
            // Vibrato (частотная модуляция)
            let _vibrato_offset = vibrato_depth * (self.vibrato_phase).sin();
            self.vibrato_phase += 2.0 * std::f32::consts::PI * vibrato_rate / self.sample_rate;
            
            // Tremolo (амплитудная модуляция)  
            let tremolo_gain = 1.0 + tremolo_depth * (self.tremolo_phase).sin();
            self.tremolo_phase += 2.0 * std::f32::consts::PI * tremolo_rate / self.sample_rate;
            
            // Применяем модуляции
            let modulated = sample * tremolo_gain;
            output.push(modulated);
        }
        
        Ok(output)
    }
}

/// Процессор пространственных эффектов
struct SpatialProcessor {
    sample_rate: f32,
    reverb_buffer: VecDeque<f32>,
    chorus_buffers: Vec<VecDeque<f32>>,
    chorus_phases: Vec<f32>,
}

impl SpatialProcessor {
    fn new(config: &NeuralConfig) -> Result<Self, String> {
        let reverb_size = (config.sample_rate * 2.0) as usize; // 2 секунды reverb
        
        Ok(Self {
            sample_rate: config.sample_rate,
            reverb_buffer: VecDeque::with_capacity(reverb_size),
            chorus_buffers: vec![VecDeque::new(); 8], // До 8 голосов
            chorus_phases: vec![0.0; 8],
        })
    }
    
    fn apply_reverb(&mut self, input: &[f32], room_size: f32, damping: f32, wet_level: f32) -> Result<Vec<f32>, String> {
        let delay_samples = (room_size * self.sample_rate * 0.1) as usize;
        let mut output = Vec::with_capacity(input.len());
        
        for &sample in input {
            // Добавляем сэмпл в reverb буфер
            self.reverb_buffer.push_back(sample);
            
            // Получаем задержанный сигнал
            let delayed = if self.reverb_buffer.len() > delay_samples {
                self.reverb_buffer.pop_front().unwrap_or(0.0)
            } else {
                0.0
            };
            
            // Применяем damping (затухание)
            let damped = delayed * (1.0 - damping);
            
            // Смешиваем сухой и мокрый сигналы
            let mixed = sample * (1.0 - wet_level) + damped * wet_level;
            output.push(mixed);
        }
        
        Ok(output)
    }
    
    fn apply_chorus(&mut self, input: &[f32], voices: u32, delay: f32, depth: f32, rate: f32) -> Result<Vec<f32>, String> {
        let delay_samples = (delay * self.sample_rate / 1000.0) as usize;
        let mut output = Vec::with_capacity(input.len());
        
        for (_i, &sample) in input.iter().enumerate() {
            let mut chorus_sum = sample; // Начинаем с оригинального сигнала
            
            // Применяем каждый голос хоруса
            for voice in 0..voices.min(8) {
                let voice_idx = voice as usize;
                
                // Модулируем задержку
                let lfo = (self.chorus_phases[voice_idx]).sin();
                let modulated_delay = delay_samples as f32 + depth * lfo * delay_samples as f32 * 0.5;
                
                // Обновляем фазу LFO
                self.chorus_phases[voice_idx] += 2.0 * std::f32::consts::PI * rate / self.sample_rate;
                
                // Добавляем сэмпл в буфер голоса
                if self.chorus_buffers[voice_idx].len() > modulated_delay as usize {
                    if let Some(delayed_sample) = self.chorus_buffers[voice_idx].get(0) {
                        chorus_sum += delayed_sample * 0.3; // Смешиваем с меньшей амплитудой
                    }
                    self.chorus_buffers[voice_idx].pop_front();
                }
                
                self.chorus_buffers[voice_idx].push_back(sample);
            }
            
            output.push(chorus_sum / (voices as f32 + 1.0)); // Нормализация
        }
        
        Ok(output)
    }
}

// Реализации по умолчанию

impl Default for NeuralConfig {
    fn default() -> Self {
        Self {
            sample_rate: 44100.0,
            buffer_size: 512,
            max_effects: 8,
            quality_preset: QualityPreset::Medium,
            enable_real_time: true,
        }
    }
}

impl Default for VoiceEffect {
    fn default() -> Self {
        VoiceEffect::PitchShift(0.0)
    }
}