use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use atomic_float::AtomicF32;
use ringbuf::HeapRb;
use crossbeam_channel::{Receiver, Sender};
use serde::{Deserialize, Serialize};

// Платформо-специфичные модули
pub mod platform;
use platform::PlatformAudio;

// AI эффекты модуль
pub mod ai_effects;
mod neural_engine;
use ai_effects::{AIProcessor, AIConfig, AIProcessingMode};
use neural_engine::{NeuralVoiceProcessor, NeuralConfig, VoiceEffect, QualityPreset, NeuralProcessingResult};

/// Статистика производительности системы
#[derive(Debug, Clone, Default)]
pub struct PerformanceStats {
    pub cpu_usage: f32,
    pub gpu_usage: f32,
    pub npu_usage: f32,
    pub memory_usage: f32,
    pub audio_latency: f32,
    pub ai_processing_time: f32,
}

/// Типы аудио эффектов
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum EffectType {
    None,
    // DSP эффекты
    Monster,     // Монстр - pitch down + distortion
    HighPitch,   // Высокий - pitch up
    Cave,        // Пещера - reverb + echo
    Radio,       // Рация - bandpass filter + distortion
    Cathedral,   // Собор - большой reverb
    Underwater,  // Под водой - lowpass + modulation
    // AI эффекты (выполняются на NPU)
    Robot,       // Робот
    Demon,       // Демон
    Alien,       // Пришелец
    // Комплексный демонстрационный эффект
    VoiceChanger, // Полная цепочка: DSP → AI → Post-processing
}

/// Типы генераторов шума
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum NoiseType {
    None,
    White,   // Белый шум
    Pink,    // Розовый шум
    Brown,   // Коричневый шум
}

/// Параметры аудио конвейера, управляемые атомарно
pub struct AudioParameters {
    // Основные параметры
    pub sample_rate: AtomicF32,
    pub buffer_size: AtomicU32,
    pub input_gain: AtomicF32,
    pub output_gain: AtomicF32,
    
    // Эффекты
    pub current_effect: AtomicU32,  // EffectType as u32
    pub effect_mix: AtomicF32,      // 0.0 - 1.0
    pub effect_bypass: AtomicBool,
    
    // Генераторы шума
    pub noise_type: AtomicU32,      // NoiseType as u32
    pub noise_level: AtomicF32,     // 0.0 - 1.0
    
    // DSP параметры
    pub pitch_shift: AtomicF32,     // 0.5 - 2.0 (полутона)
    pub reverb_size: AtomicF32,     // 0.0 - 1.0
    pub reverb_damping: AtomicF32,  // 0.0 - 1.0
    pub delay_time: AtomicF32,      // 0.0 - 1.0 секунд
    pub delay_feedback: AtomicF32,  // 0.0 - 0.95
    
    // Фильтры
    pub lowpass_freq: AtomicF32,    // 20 - 20000 Hz
    pub highpass_freq: AtomicF32,   // 20 - 20000 Hz
    pub bandpass_center: AtomicF32, // 100 - 8000 Hz
    pub bandpass_q: AtomicF32,      // 0.1 - 10.0
}

impl Default for AudioParameters {
    fn default() -> Self {
        Self {
            sample_rate: AtomicF32::new(44100.0),
            buffer_size: AtomicU32::new(512),
            input_gain: AtomicF32::new(1.0),
            output_gain: AtomicF32::new(1.0),
            current_effect: AtomicU32::new(EffectType::None as u32),
            effect_mix: AtomicF32::new(1.0),
            effect_bypass: AtomicBool::new(false),
            noise_type: AtomicU32::new(NoiseType::None as u32),
            noise_level: AtomicF32::new(0.0),
            pitch_shift: AtomicF32::new(1.0),
            reverb_size: AtomicF32::new(0.5),
            reverb_damping: AtomicF32::new(0.5),
            delay_time: AtomicF32::new(0.3),
            delay_feedback: AtomicF32::new(0.3),
            lowpass_freq: AtomicF32::new(20000.0),
            highpass_freq: AtomicF32::new(20.0),
            bandpass_center: AtomicF32::new(1000.0),
            bandpass_q: AtomicF32::new(1.0),
        }
    }
}

/// Простой генератор шума
pub struct NoiseGenerator {
    pub noise_type: NoiseType,
    pub level: f32,
    // Состояние для различных типов шума
    white_state: u32,
    pink_state: [f32; 7],
    brown_state: f32,
}

impl NoiseGenerator {
    pub fn new() -> Self {
        Self {
            noise_type: NoiseType::None,
            level: 0.0,
            white_state: 12345,
            pink_state: [0.0; 7],
            brown_state: 0.0,
        }
    }
    
    pub fn generate_sample(&mut self) -> f32 {
        if self.level <= 0.0 {
            return 0.0;
        }
        
        let noise = match self.noise_type {
            NoiseType::None => 0.0,
            NoiseType::White => self.white_noise(),
            NoiseType::Pink => self.pink_noise(),
            NoiseType::Brown => self.brown_noise(),
        };
        
        noise * self.level
    }
    
    fn white_noise(&mut self) -> f32 {
        // Простой LCPRNG для белого шума
        self.white_state = self.white_state.wrapping_mul(1103515245).wrapping_add(12345);
        (self.white_state as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
    
    fn pink_noise(&mut self) -> f32 {
        let white = self.white_noise();
        self.pink_state[0] = 0.99886 * self.pink_state[0] + white * 0.0555179;
        self.pink_state[1] = 0.99332 * self.pink_state[1] + white * 0.0750759;
        self.pink_state[2] = 0.96900 * self.pink_state[2] + white * 0.1538520;
        self.pink_state[3] = 0.86650 * self.pink_state[3] + white * 0.3104856;
        self.pink_state[4] = 0.55000 * self.pink_state[4] + white * 0.5329522;
        self.pink_state[5] = -0.7616 * self.pink_state[5] - white * 0.0168980;
        let output = self.pink_state[0] + self.pink_state[1] + self.pink_state[2] + 
                    self.pink_state[3] + self.pink_state[4] + self.pink_state[5] + 
                    self.pink_state[6] + white * 0.5362;
        self.pink_state[6] = white * 0.115926;
        output * 0.11
    }
    
    fn brown_noise(&mut self) -> f32 {
        let white = self.white_noise();
        self.brown_state = (self.brown_state + white * 0.02).clamp(-1.0, 1.0);
        self.brown_state * 3.5
    }
}

/// Простой delay эффект
pub struct DelayEffect {
    buffer: Vec<f32>,
    write_pos: usize,
    delay_samples: usize,
    feedback: f32,
    mix: f32,
}

impl DelayEffect {
    pub fn new(max_delay_samples: usize) -> Self {
        Self {
            buffer: vec![0.0; max_delay_samples],
            write_pos: 0,
            delay_samples: max_delay_samples / 4,
            feedback: 0.3,
            mix: 0.3,
        }
    }
    
    pub fn process(&mut self, input: f32) -> f32 {
        let read_pos = (self.write_pos + self.buffer.len() - self.delay_samples) % self.buffer.len();
        let delayed = self.buffer[read_pos];
        
        self.buffer[self.write_pos] = input + delayed * self.feedback;
        self.write_pos = (self.write_pos + 1) % self.buffer.len();
        
        input + delayed * self.mix
    }
    
    pub fn set_delay_time(&mut self, time_sec: f32, sample_rate: f32) {
        self.delay_samples = ((time_sec * sample_rate) as usize).min(self.buffer.len() - 1);
    }
    
    pub fn set_feedback(&mut self, feedback: f32) {
        self.feedback = feedback.clamp(0.0, 0.95);
    }
    
    pub fn set_mix(&mut self, mix: f32) {
        self.mix = mix.clamp(0.0, 1.0);
    }
}

/// Простой biquad фильтр
pub struct BiquadFilter {
    b0: f32, b1: f32, b2: f32,
    a1: f32, a2: f32,
    x1: f32, x2: f32,
    y1: f32, y2: f32,
}

impl BiquadFilter {
    pub fn new() -> Self {
        Self {
            b0: 1.0, b1: 0.0, b2: 0.0,
            a1: 0.0, a2: 0.0,
            x1: 0.0, x2: 0.0,
            y1: 0.0, y2: 0.0,
        }
    }
    
    pub fn lowpass(&mut self, freq: f32, sample_rate: f32, q: f32) {
        let omega = 2.0 * std::f32::consts::PI * freq / sample_rate;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let alpha = sin_omega / (2.0 * q);
        
        let b0 = (1.0 - cos_omega) / 2.0;
        let b1 = 1.0 - cos_omega;
        let b2 = (1.0 - cos_omega) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_omega;
        let a2 = 1.0 - alpha;
        
        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
    }
    
    pub fn highpass(&mut self, freq: f32, sample_rate: f32, q: f32) {
        let omega = 2.0 * std::f32::consts::PI * freq / sample_rate;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let alpha = sin_omega / (2.0 * q);
        
        let b0 = (1.0 + cos_omega) / 2.0;
        let b1 = -(1.0 + cos_omega);
        let b2 = (1.0 + cos_omega) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_omega;
        let a2 = 1.0 - alpha;
        
        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
    }
    
    pub fn bandpass(&mut self, freq: f32, sample_rate: f32, q: f32) {
        let omega = 2.0 * std::f32::consts::PI * freq / sample_rate;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let alpha = sin_omega / (2.0 * q);
        
        let b0 = alpha;
        let b1 = 0.0;
        let b2 = -alpha;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_omega;
        let a2 = 1.0 - alpha;
        
        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
    }
    
    pub fn process(&mut self, input: f32) -> f32 {
        let output = self.b0 * input + self.b1 * self.x1 + self.b2 * self.x2
                   - self.a1 * self.y1 - self.a2 * self.y2;
        
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;
        
        output
    }
}

/// DSP процессор
pub struct DspProcessor {
    pub delay: DelayEffect,
    pub lowpass: BiquadFilter,
    pub highpass: BiquadFilter,
    pub bandpass: BiquadFilter,
    pub sample_rate: f32,
}

impl DspProcessor {
    pub fn new(sample_rate: f32, max_delay_samples: usize) -> Self {
        Self {
            delay: DelayEffect::new(max_delay_samples),
            lowpass: BiquadFilter::new(),
            highpass: BiquadFilter::new(),
            bandpass: BiquadFilter::new(),
            sample_rate,
        }
    }
    
    pub fn process_effect(&mut self, input: f32, effect_type: EffectType, params: &AudioParameters) -> f32 {
        match effect_type {
            EffectType::None => input,
            
            EffectType::Monster => {
                // Монстр: понижение тона + искажение
                let pitched = input * 0.5; // Простое понижение
                let distorted = (pitched * 3.0).tanh(); // Мягкое искажение
                distorted * 0.8
            },
            
            EffectType::HighPitch => {
                // Высокий тон: повышение частоты
                input * 1.5 // Простое повышение
            },
            
            EffectType::Cave => {
                // Пещера: эхо + реверб
                self.delay.set_delay_time(params.delay_time.load(Ordering::Relaxed), self.sample_rate);
                self.delay.set_feedback(0.6);
                self.delay.set_mix(0.4);
                self.delay.process(input)
            },
            
            EffectType::Radio => {
                // Рация: полосовой фильтр + искажение
                self.bandpass.bandpass(
                    params.bandpass_center.load(Ordering::Relaxed),
                    self.sample_rate,
                    2.0
                );
                let filtered = self.bandpass.process(input);
                (filtered * 2.0).tanh() * 0.7
            },
            
            EffectType::Cathedral => {
                // Собор: большой реверб
                self.delay.set_delay_time(0.8, self.sample_rate);
                self.delay.set_feedback(0.7);
                self.delay.set_mix(0.6);
                self.delay.process(input)
            },
            
            EffectType::Underwater => {
                // Под водой: низкие частоты + модуляция
                self.lowpass.lowpass(800.0, self.sample_rate, 1.0);
                self.lowpass.process(input) * 0.8
            },
            
            // AI эффекты - заглушки (в реальности будут обрабатываться через Core ML)
            EffectType::Robot | EffectType::Demon | EffectType::Alien | EffectType::VoiceChanger => {
                // Для AI эффектов возвращаем входной сигнал
                // В реальной реализации здесь будет вызов AI модели
                input
            },
        }
    }
}

/// Главная структура аудио конвейера
pub struct AudioPipeline {
    pub parameters: AudioParameters,
    pub noise_generator: NoiseGenerator,
    pub dsp_processor: DspProcessor,
    
    // AI процессор для NPU обработки
    pub ai_processor: AIProcessor,
    
    // Neural Engine процессор (Apple Silicon M1/M2/M3)
    pub neural_processor: Option<NeuralVoiceProcessor>,
    
    // Буферы для обработки
    pub input_buffer: HeapRb<f32>,
    pub output_buffer: HeapRb<f32>,
    
    // Каналы для коммуникации с AI процессором
    pub ai_input_sender: Option<Sender<Vec<f32>>>,
    pub ai_output_receiver: Option<Receiver<Vec<f32>>>,
    
    // Платформо-специфичная аудио подсистема
    pub platform_audio: Option<platform::PlatformAudioImpl>,
    
    // Счетчики и статистика
    pub samples_processed: u64,
    pub is_processing: AtomicBool,
    
    // Статистика производительности
    pub performance_stats: PerformanceStats,
}

impl AudioPipeline {
    pub fn new(sample_rate: f32, buffer_size: usize) -> Self {
        let max_delay_samples = (sample_rate * 2.0) as usize; // 2 секунды максимум
        
        // Создаем конфигурацию для AI процессора
        let ai_config = AIConfig {
            sample_rate,
            buffer_size,
            model_path: None,
            use_npu: true,
            processing_mode: AIProcessingMode::Balanced,
        };
        
        Self {
            parameters: AudioParameters::default(),
            noise_generator: NoiseGenerator::new(),
            dsp_processor: DspProcessor::new(sample_rate, max_delay_samples),
            ai_processor: AIProcessor::new(ai_config),
        
        // Инициализируем Neural Engine процессор на Apple Silicon
        neural_processor: {
            #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
            {
                let neural_config = NeuralConfig {
                    sample_rate: params.sample_rate,
                    buffer_size: params.buffer_size,
                    max_effects: 8,
                    quality_preset: QualityPreset::Medium,
                    enable_real_time: true,
                };
                
                match NeuralVoiceProcessor::new(neural_config) {
                    Ok(processor) => {
                        println!("✅ Neural Engine процессор инициализирован");
                        Some(processor)
                    }
                    Err(e) => {
                        println!("⚠️ Не удалось инициализировать Neural Engine: {}", e);
                        None
                    }
                }
            }
            #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
            {
                None
            }
        },
            input_buffer: HeapRb::new(buffer_size * 4),
            output_buffer: HeapRb::new(buffer_size * 4),
            ai_input_sender: None,
            ai_output_receiver: None,
            platform_audio: None,
            samples_processed: 0,
            is_processing: AtomicBool::new(false),
            performance_stats: PerformanceStats::default(),
        }
    }
    
    /// Создает конвейер с автоматической инициализацией платформы
    pub fn new_with_platform() -> Result<Self, Box<dyn std::error::Error>> {
        use platform::{PlatformAudio, PlatformAudioImpl};
        
        let mut pipeline = Self::new(44100.0, 512);
        
        match PlatformAudioImpl::initialize() {
            Ok(platform_audio) => {
                let sample_rate = platform_audio.get_sample_rate();
                let buffer_size = platform_audio.get_buffer_size();
                
                println!("🎯 Платформа инициализирована: {}", platform_audio.platform_info());
                
                // Обновляем параметры на основе возможностей платформы
                pipeline.parameters.sample_rate.store(sample_rate, Ordering::Relaxed);
                pipeline.parameters.buffer_size.store(buffer_size as u32, Ordering::Relaxed);
                pipeline.platform_audio = Some(platform_audio);
                
                Ok(pipeline)
            }
            Err(e) => {
                println!("⚠️  Не удалось инициализировать платформу: {}", e);
                println!("ℹ️  Используется базовая реализация без платформо-специфичных оптимизаций");
                Ok(pipeline)
            }
        }
    }
    
    pub fn process_block(&mut self, input: &[f32], output: &mut [f32]) {
        if !self.is_processing.load(Ordering::Relaxed) {
            // Если обработка отключена, заполняем тишиной
            output.fill(0.0);
            return;
        }
        
        let input_gain = self.parameters.input_gain.load(Ordering::Relaxed);
        let output_gain = self.parameters.output_gain.load(Ordering::Relaxed);
        let effect_type_raw = self.parameters.current_effect.load(Ordering::Relaxed);
        let effect_mix = self.parameters.effect_mix.load(Ordering::Relaxed);
        let effect_bypass = self.parameters.effect_bypass.load(Ordering::Relaxed);
        
        // Преобразуем u32 обратно в enum
        let effect_type = match effect_type_raw {
            0 => EffectType::None,
            1 => EffectType::Monster,
            2 => EffectType::HighPitch,
            3 => EffectType::Cave,
            4 => EffectType::Radio,
            5 => EffectType::Cathedral,
            6 => EffectType::Underwater,
            7 => EffectType::Robot,
            8 => EffectType::Demon,
            9 => EffectType::Alien,
            10 => EffectType::VoiceChanger,
            _ => EffectType::None,
        };
        
        // Обновляем параметры генератора шума
        let noise_type_raw = self.parameters.noise_type.load(Ordering::Relaxed);
        self.noise_generator.noise_type = match noise_type_raw {
            0 => NoiseType::None,
            1 => NoiseType::White,
            2 => NoiseType::Pink,
            3 => NoiseType::Brown,
            _ => NoiseType::None,
        };
        self.noise_generator.level = self.parameters.noise_level.load(Ordering::Relaxed);
        
        // Для AI эффектов обрабатываем весь блок сразу
        if matches!(effect_type, EffectType::Robot | EffectType::Demon | EffectType::Alien | EffectType::VoiceChanger) && !effect_bypass {
            // Подготавливаем входной буфер для AI
            let mut ai_input = Vec::with_capacity(input.len());
            for &input_sample in input.iter() {
                let mut sample = input_sample * input_gain;
                sample += self.noise_generator.generate_sample();
                ai_input.push(sample);
            }
            
            // Обрабатываем через AI
            let ai_result = self.ai_processor.process(&ai_input);
            let mut processed_output = ai_result.output.clone();
            
            // Neural Engine обработка (Apple Silicon M1/M2/M3)
            if let Some(ref mut neural) = self.neural_processor {
                match neural.process(&processed_output) {
                    Ok(neural_result) => {
                        processed_output = neural_result.output;
                        // Логируем только значительную активность Neural Engine
                        if neural_result.neural_engine_load > 10.0 {
                            println!("🧠 Neural Engine: Нагрузка {:.1}%, Задержка {:.0} нс, Качество {:.2}", 
                                neural_result.neural_engine_load, 
                                neural_result.latency_ns,
                                neural_result.quality_score);
                        }
                    }
                    Err(e) => {
                        println!("⚠️ Ошибка Neural Engine: {}", e);
                    }
                }
            }
            
            // Обновляем статистику производительности
            self.performance_stats.ai_processing_time = ai_result.latency_ms;
            self.performance_stats.npu_usage = ai_result.npu_utilization;
            
            // Для VoiceChanger применяем дополнительную DSP обработку
            if effect_type == EffectType::VoiceChanger {
                for (i, &ai_sample) in processed_output.iter().enumerate() {
                    if i >= output.len() { break; }
                    let dsp_processed = self.dsp_processor.process_effect(ai_sample, EffectType::Cave, &self.parameters);
                    let mixed = ai_sample * (1.0 - effect_mix) + dsp_processed * effect_mix;
                    output[i] = mixed * output_gain;
                }
            } else {
                // Для других AI эффектов применяем только микс
                for (i, &ai_sample) in processed_output.iter().enumerate() {
                    if i >= output.len() { break; }
                    output[i] = ai_sample * output_gain;
                }
            }
        } else {
            // Обычная DSP обработка для не-AI эффектов
            for (i, &input_sample) in input.iter().enumerate() {
                if i >= output.len() { break; }
                
                // Применяем входной усилитель
                let mut sample = input_sample * input_gain;
                
                // Добавляем шум
                sample += self.noise_generator.generate_sample();
                
                // Применяем эффект (если не в bypass режиме)
                if !effect_bypass {
                    let processed = self.dsp_processor.process_effect(sample, effect_type, &self.parameters);
                    sample = sample * (1.0 - effect_mix) + processed * effect_mix;
                }
                
                // Применяем выходной усилитель и записываем
                output[i] = sample * output_gain;
            }
        }
        
        self.samples_processed += input.len() as u64;
    }
    
    pub fn set_effect(&mut self, effect: EffectType) {
        self.parameters.current_effect.store(effect as u32, Ordering::Relaxed);
    }
    
    pub fn set_noise(&mut self, noise_type: NoiseType, level: f32) {
        self.parameters.noise_type.store(noise_type as u32, Ordering::Relaxed);
        self.parameters.noise_level.store(level.clamp(0.0, 1.0), Ordering::Relaxed);
    }
    
    pub fn start_processing(&mut self) {
        self.is_processing.store(true, Ordering::Relaxed);
    }
    
    pub fn stop_processing(&mut self) {
        self.is_processing.store(false, Ordering::Relaxed);
    }
    
    /// Получает информацию о платформе
    pub fn platform_info(&self) -> String {
        if let Some(ref platform) = self.platform_audio {
            platform.platform_info()
        } else {
            "Базовая реализация (без платформо-специфичных оптимизаций)".to_string()
        }
    }
    
    /// Проверяет поддержку низкой задержки
    pub fn supports_low_latency(&self) -> bool {
        self.platform_audio
            .as_ref()
            .map(|p| p.supports_low_latency())
            .unwrap_or(false)
    }
    
    /// Проверяет, поддерживается ли NPU (только для macOS Apple Silicon)
    #[cfg(target_os = "macos")]
    pub fn supports_neural_engine(&self) -> bool {
        if let Some(ref platform_audio) = self.platform_audio {
            platform_audio.supports_neural_engine()
        } else {
            self.ai_processor.supports_npu()
        }
    }
    
    /// Заглушка для не-macOS платформ
    #[cfg(not(target_os = "macos"))]
    pub fn supports_neural_engine(&self) -> bool {
        self.ai_processor.supports_npu()
    }
    
    /// Получает статистику производительности
    pub fn get_performance_stats(&self) -> PerformanceStats {
        let mut stats = self.performance_stats.clone();
        stats.npu_usage = self.ai_processor.get_average_npu_load();
        stats.ai_processing_time = self.ai_processor.get_average_latency();
        stats
    }
    
    /// Получает детальную информацию о системе
    pub fn get_system_info(&self) -> String {
        let neural_info = if self.neural_processor.is_some() {
            format!("\n🧠 Neural Engine: {}\n💫 Neural задержка: {:.0} нс\n🎛️ Neural нагрузка: {:.1}%",
                self.neural_engine_info(),
                self.get_neural_latency_ns(),
                self.get_neural_load())
        } else {
            String::new()
        };
        
        format!(
            "🎯 Аудио конвейер\n\
             📊 Частота дискретизации: {:.0} Гц\n\
             🔧 Размер буфера: {} сэмплов\n\
             🎵 Обработано сэмплов: {}\n\
             🧠 NPU поддержка: {}\n\
             📈 Средняя задержка AI: {:.2} мс\n\
             💻 Средняя нагрузка NPU: {:.1}%{}\n\
             🔧 {}",
            self.parameters.sample_rate.load(Ordering::Relaxed),
            self.parameters.buffer_size.load(Ordering::Relaxed),
            self.samples_processed,
            if self.supports_neural_engine() { "✅ Да" } else { "❌ Нет" },
            self.ai_processor.get_average_latency(),
            self.ai_processor.get_average_npu_load(),
            neural_info,
            self.platform_info()
        )
    }
    
    // === Neural Engine методы ===
    
    /// Добавляет голосовой эффект в Neural Engine
    pub fn add_voice_effect(&mut self, effect: VoiceEffect) -> Result<(), String> {
        if let Some(ref mut neural) = self.neural_processor {
            neural.add_effect(effect)
        } else {
            Err("Neural Engine недоступен".to_string())
        }
    }
    
    /// Удаляет голосовой эффект из Neural Engine
    pub fn remove_voice_effect(&mut self, effect: &VoiceEffect) {
        if let Some(ref mut neural) = self.neural_processor {
            neural.remove_effect(effect);
        }
    }
    
    /// Очищает все голосовые эффекты
    pub fn clear_voice_effects(&mut self) {
        if let Some(ref mut neural) = self.neural_processor {
            neural.clear_effects();
        }
    }
    
    /// Возвращает информацию о Neural Engine
    pub fn neural_engine_info(&self) -> String {
        if let Some(ref neural) = self.neural_processor {
            neural.neural_engine_info()
        } else {
            "Neural Engine недоступен".to_string()
        }
    }
    
    /// Возвращает задержку Neural Engine в наносекундах
    pub fn get_neural_latency_ns(&self) -> u64 {
        if let Some(ref neural) = self.neural_processor {
            neural.get_average_latency_ns()
        } else {
            0
        }
    }
    
    /// Возвращает нагрузку на Neural Engine
    pub fn get_neural_load(&self) -> f32 {
        if let Some(ref neural) = self.neural_processor {
            neural.get_average_neural_load()
        } else {
            0.0
        }
    }
}

/// Создает экземпляр аудиоконвейера и возвращает указатель на него.
#[no_mangle]
pub extern "C" fn create_pipeline() -> *mut c_void {
    println!("Rust: create_pipeline() вызван.");
    let pipeline = Box::new(AudioPipeline::new(44100.0, 512));
    Box::into_raw(pipeline) as *mut c_void
}

/// Создает экземпляр аудиоконвейера с платформо-специфичной инициализацией.
#[no_mangle]
pub extern "C" fn create_pipeline_with_platform() -> *mut c_void {
    println!("Rust: create_pipeline_with_platform() вызван.");
    match AudioPipeline::new_with_platform() {
        Ok(pipeline) => {
            println!("Rust: Платформа успешно инициализирована");
            Box::into_raw(Box::new(pipeline)) as *mut c_void
        }
        Err(e) => {
            println!("Rust: Ошибка инициализации платформы: {}", e);
            // Возвращаем базовую реализацию
            let pipeline = Box::new(AudioPipeline::new(44100.0, 512));
            Box::into_raw(pipeline) as *mut c_void
        }
    }
}

/// Обрабатывает блок аудиоданных.
///
/// # Safety
/// Эта функция небезопасна, так как работает с сырыми указателями из C.
#[no_mangle]
pub unsafe extern "C" fn process_audio(
    pipeline_ptr: *mut c_void,
    input: *const f32,
    output: *mut f32,
    len: usize,
) {
    if pipeline_ptr.is_null() {
        return;
    }
    
    let pipeline = &mut *(pipeline_ptr as *mut AudioPipeline);
    let input_slice = std::slice::from_raw_parts(input, len);
    let output_slice = std::slice::from_raw_parts_mut(output, len);
    
    pipeline.process_block(input_slice, output_slice);
}

/// Устанавливает эффект
#[no_mangle]
pub unsafe extern "C" fn set_effect(pipeline_ptr: *mut c_void, effect_type: u32) {
    if pipeline_ptr.is_null() { return; }
    let pipeline = &mut *(pipeline_ptr as *mut AudioPipeline);
    pipeline.parameters.current_effect.store(effect_type, Ordering::Relaxed);
}

/// Устанавливает параметры шума
#[no_mangle]
pub unsafe extern "C" fn set_noise(pipeline_ptr: *mut c_void, noise_type: u32, level: f32) {
    if pipeline_ptr.is_null() { return; }
    let pipeline = &mut *(pipeline_ptr as *mut AudioPipeline);
    pipeline.parameters.noise_type.store(noise_type, Ordering::Relaxed);
    pipeline.parameters.noise_level.store(level.clamp(0.0, 1.0), Ordering::Relaxed);
}

/// Запускает обработку
#[no_mangle]
pub unsafe extern "C" fn start_processing(pipeline_ptr: *mut c_void) {
    if pipeline_ptr.is_null() { return; }
    let pipeline = &mut *(pipeline_ptr as *mut AudioPipeline);
    pipeline.start_processing();
}

/// Останавливает обработку
#[no_mangle]
pub unsafe extern "C" fn stop_processing(pipeline_ptr: *mut c_void) {
    if pipeline_ptr.is_null() { return; }
    let pipeline = &mut *(pipeline_ptr as *mut AudioPipeline);
    pipeline.stop_processing();
}

/// Получает загрузку NPU (возвращает процент 0.0-100.0)
#[no_mangle]
pub unsafe extern "C" fn get_npu_load(pipeline_ptr: *mut c_void) -> f32 {
    if pipeline_ptr.is_null() { return 0.0; }
    let pipeline = &*(pipeline_ptr as *mut AudioPipeline);
    pipeline.ai_processor.get_average_npu_load()
}

/// Получает задержку AI обработки в миллисекундах
#[no_mangle]
pub unsafe extern "C" fn get_ai_latency(pipeline_ptr: *mut c_void) -> f32 {
    if pipeline_ptr.is_null() { return 0.0; }
    let pipeline = &*(pipeline_ptr as *mut AudioPipeline);
    pipeline.ai_processor.get_average_latency()
}

/// Проверяет поддержку NPU
#[no_mangle]
pub unsafe extern "C" fn supports_npu(pipeline_ptr: *mut c_void) -> bool {
    if pipeline_ptr.is_null() { return false; }
    let pipeline = &*(pipeline_ptr as *mut AudioPipeline);
    pipeline.supports_neural_engine()
}

/// Освобождает память, выделенную под аудиоконвейер.
///
/// # Safety
/// Эта функция небезопасна, так как работает с сырыми указателями из C.
#[no_mangle]
pub unsafe extern "C" fn destroy_pipeline(pipeline_ptr: *mut c_void) {
    if !pipeline_ptr.is_null() {
        println!("Rust: destroy_pipeline() вызван.");
        drop(Box::from_raw(pipeline_ptr as *mut AudioPipeline));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_pipeline_creation() {
        let pipeline = AudioPipeline::new(44100.0, 512);
        assert_eq!(pipeline.samples_processed, 0);
        assert!(!pipeline.is_processing.load(Ordering::Relaxed));
    }
    
    #[test]
    fn test_platform_initialization() {
        // Тестируем создание конвейера с платформой
        let result = AudioPipeline::new_with_platform();
        assert!(result.is_ok(), "Не удалось создать конвейер с платформой");
        
        let pipeline = result.unwrap();
        println!("Платформа: {}", pipeline.platform_info());
        println!("Поддержка низкой задержки: {}", pipeline.supports_low_latency());
        println!("Поддержка Neural Engine: {}", pipeline.supports_neural_engine());
    }

    #[test]
    fn test_effects() {
        let mut pipeline = AudioPipeline::new(44100.0, 512);
        pipeline.start_processing();
        
        // Тестируем различные эффекты
        let effects = [
            EffectType::None,
            EffectType::Monster,
            EffectType::HighPitch,
            EffectType::Cave,
            EffectType::Radio,
            EffectType::Cathedral,
            EffectType::Underwater,
            EffectType::Robot,
            EffectType::Demon,
            EffectType::Alien,
        ];
        
        // Создаем тестовый сигнал
        let input = vec![0.5f32; 100];
        let mut output = vec![0.0f32; 100];
        
        for effect in effects.iter() {
            pipeline.set_effect(*effect);
            pipeline.process_block(&input, &mut output);
            
            // Проверяем, что выходной сигнал был изменен
            assert!(output.iter().any(|&x| x != 0.0));
        }
    }

    #[test]
    fn test_noise_generators() {
        let mut pipeline = AudioPipeline::new(44100.0, 512);
        pipeline.start_processing();
        
        let noise_types = [NoiseType::White, NoiseType::Pink, NoiseType::Brown];
        let input = vec![0.0f32; 100]; // Тишина на входе
        let mut output = vec![0.0f32; 100];
        
        for noise_type in noise_types.iter() {
            pipeline.set_noise(*noise_type, 0.1);
            pipeline.process_block(&input, &mut output);
            
            // Проверяем, что был добавлен шум
            let rms = calculate_rms(&output);
            assert!(rms > 0.0, "Шум не был добавлен для {:?}", noise_type);
        }
    }

    #[test]
    fn test_filters() {
        let mut filter = BiquadFilter::new();
        
        // Тест низкочастотного фильтра
        filter.lowpass(1000.0, 44100.0, 1.0);
        let output = filter.process(1.0);
        assert!(output.is_finite());
        
        // Тест высокочастотного фильтра
        filter.highpass(1000.0, 44100.0, 1.0);
        let output = filter.process(1.0);
        assert!(output.is_finite());
        
        // Тест полосового фильтра
        filter.bandpass(1000.0, 44100.0, 1.0);
        let output = filter.process(1.0);
        assert!(output.is_finite());
    }

    #[test]
    fn test_delay_effect() {
        let mut delay = DelayEffect::new(4410); // 100 мс при 44100 Гц
        delay.set_delay_time(0.1, 44100.0);
        delay.set_feedback(0.5);
        delay.set_mix(0.3);
        
        let output = delay.process(1.0);
        assert!(output.is_finite());
    }

    #[test]
    fn test_noise_generator() {
        let mut generator = NoiseGenerator::new();
        
        // Тест белого шума
        generator.noise_type = NoiseType::White;
        generator.level = 1.0;
        let white_noise = generator.generate_sample();
        assert!(white_noise.is_finite());
        
        // Тест розового шума
        generator.noise_type = NoiseType::Pink;
        let pink_noise = generator.generate_sample();
        assert!(pink_noise.is_finite());
        
        // Тест коричневого шума
        generator.noise_type = NoiseType::Brown;
        let brown_noise = generator.generate_sample();
        assert!(brown_noise.is_finite());
    }

    #[test]
    fn test_c_ffi_interface() {
        unsafe {
            // Создаем конвейер через C интерфейс
            let pipeline_ptr = create_pipeline();
            assert!(!pipeline_ptr.is_null());
            
            // Тестируем установку эффекта
            set_effect(pipeline_ptr, EffectType::Monster as u32);
            
            // Тестируем установку шума
            set_noise(pipeline_ptr, NoiseType::White as u32, 0.1);
            
            // Тестируем запуск обработки
            start_processing(pipeline_ptr);
            
            // Тестируем обработку аудио
            let input = vec![0.5f32; 10];
            let mut output = vec![0.0f32; 10];
            process_audio(pipeline_ptr, input.as_ptr(), output.as_mut_ptr(), 10);
            
            // Тестируем остановку обработки
            stop_processing(pipeline_ptr);
            
            // Освобождаем память
            destroy_pipeline(pipeline_ptr);
        }
    }

    /// Вычисляет RMS (среднеквадратичное значение) сигнала
    fn calculate_rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        
        let sum_squares: f32 = samples.iter().map(|&x| x * x).sum();
        (sum_squares / samples.len() as f32).sqrt()
    }
}

// === Neural Engine C API ===

/// Добавляет эффект изменения высоты тона
#[no_mangle]
pub extern "C" fn add_pitch_shift_effect(pipeline_ptr: *mut c_void, semitones: f32) -> i32 {
    if pipeline_ptr.is_null() {
        return -1;
    }
    
    unsafe {
        let pipeline = &mut *(pipeline_ptr as *mut AudioPipeline);
        match pipeline.add_voice_effect(VoiceEffect::PitchShift(semitones)) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    }
}

/// Добавляет эффект изменения формант
#[no_mangle]
pub extern "C" fn add_formant_shift_effect(pipeline_ptr: *mut c_void, shift: f32) -> i32 {
    if pipeline_ptr.is_null() {
        return -1;
    }
    
    unsafe {
        let pipeline = &mut *(pipeline_ptr as *mut AudioPipeline);
        match pipeline.add_voice_effect(VoiceEffect::FormantShift(shift)) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    }
}

/// Добавляет эффект изменения голоса
#[no_mangle]
pub extern "C" fn add_voice_changer_effect(pipeline_ptr: *mut c_void, gender: f32, age: f32, roughness: f32) -> i32 {
    if pipeline_ptr.is_null() {
        return -1;
    }
    
    unsafe {
        let pipeline = &mut *(pipeline_ptr as *mut AudioPipeline);
        match pipeline.add_voice_effect(VoiceEffect::VoiceChanger { gender, age, roughness }) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    }
}

/// Добавляет гармонические эффекты
#[no_mangle]
pub extern "C" fn add_harmonics_effect(pipeline_ptr: *mut c_void, overtones: f32, undertones: f32, distortion: f32) -> i32 {
    if pipeline_ptr.is_null() {
        return -1;
    }
    
    unsafe {
        let pipeline = &mut *(pipeline_ptr as *mut AudioPipeline);
        match pipeline.add_voice_effect(VoiceEffect::Harmonics { overtones, undertones, distortion }) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    }
}

/// Добавляет модуляционные эффекты
#[no_mangle]
pub extern "C" fn add_modulation_effect(pipeline_ptr: *mut c_void, vibrato_rate: f32, vibrato_depth: f32, tremolo_rate: f32, tremolo_depth: f32) -> i32 {
    if pipeline_ptr.is_null() {
        return -1;
    }
    
    unsafe {
        let pipeline = &mut *(pipeline_ptr as *mut AudioPipeline);
        match pipeline.add_voice_effect(VoiceEffect::Modulation { vibrato_rate, vibrato_depth, tremolo_rate, tremolo_depth }) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    }
}

/// Добавляет эффект реверберации
#[no_mangle]
pub extern "C" fn add_reverb_effect(pipeline_ptr: *mut c_void, room_size: f32, damping: f32, wet_level: f32) -> i32 {
    if pipeline_ptr.is_null() {
        return -1;
    }
    
    unsafe {
        let pipeline = &mut *(pipeline_ptr as *mut AudioPipeline);
        match pipeline.add_voice_effect(VoiceEffect::Reverb { room_size, damping, wet_level }) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    }
}

/// Добавляет эффект хоруса
#[no_mangle]
pub extern "C" fn add_chorus_effect(pipeline_ptr: *mut c_void, voices: u32, delay: f32, depth: f32, rate: f32) -> i32 {
    if pipeline_ptr.is_null() {
        return -1;
    }
    
    unsafe {
        let pipeline = &mut *(pipeline_ptr as *mut AudioPipeline);
        match pipeline.add_voice_effect(VoiceEffect::Chorus { voices, delay, depth, rate }) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    }
}

/// Добавляет эффект искажения
#[no_mangle]
pub extern "C" fn add_distortion_effect(pipeline_ptr: *mut c_void, drive: f32, tone: f32, level: f32) -> i32 {
    if pipeline_ptr.is_null() {
        return -1;
    }
    
    unsafe {
        let pipeline = &mut *(pipeline_ptr as *mut AudioPipeline);
        match pipeline.add_voice_effect(VoiceEffect::Distortion { drive, tone, level }) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    }
}

/// Добавляет эффект автотюна
#[no_mangle]
pub extern "C" fn add_autotune_effect(pipeline_ptr: *mut c_void, correction: f32, speed: f32, key: i32) -> i32 {
    if pipeline_ptr.is_null() {
        return -1;
    }
    
    unsafe {
        let pipeline = &mut *(pipeline_ptr as *mut AudioPipeline);
        match pipeline.add_voice_effect(VoiceEffect::AutoTune { correction, speed, key }) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    }
}

/// Очищает все голосовые эффекты
#[no_mangle]
pub extern "C" fn clear_voice_effects(pipeline_ptr: *mut c_void) {
    if pipeline_ptr.is_null() {
        return;
    }
    
    unsafe {
        let pipeline = &mut *(pipeline_ptr as *mut AudioPipeline);
        pipeline.clear_voice_effects();
    }
}

/// Возвращает нагрузку на Neural Engine (0.0-100.0)
#[no_mangle]
pub extern "C" fn get_neural_load(pipeline_ptr: *const c_void) -> f32 {
    if pipeline_ptr.is_null() {
        return 0.0;
    }
    
    unsafe {
        let pipeline = &*(pipeline_ptr as *const AudioPipeline);
        pipeline.get_neural_load()
    }
}

/// Возвращает задержку Neural Engine в наносекундах
#[no_mangle]
pub extern "C" fn get_neural_latency_ns(pipeline_ptr: *const c_void) -> u64 {
    if pipeline_ptr.is_null() {
        return 0;
    }
    
    unsafe {
        let pipeline = &*(pipeline_ptr as *const AudioPipeline);
        pipeline.get_neural_latency_ns()
    }
}
