use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use atomic_float::AtomicF32;
use ringbuf::HeapRb;
use crossbeam_channel::{Receiver, Sender};

/// Типы аудио эффектов
#[derive(Debug, Clone, Copy, PartialEq)]
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
}

/// Типы генераторов шума
#[derive(Debug, Clone, Copy, PartialEq)]
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
            EffectType::Robot | EffectType::Demon | EffectType::Alien => {
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
    
    // Буферы для обработки
    pub input_buffer: HeapRb<f32>,
    pub output_buffer: HeapRb<f32>,
    
    // Каналы для коммуникации с AI процессором
    pub ai_input_sender: Option<Sender<Vec<f32>>>,
    pub ai_output_receiver: Option<Receiver<Vec<f32>>>,
    
    // Счетчики и статистика
    pub samples_processed: u64,
    pub is_processing: AtomicBool,
}

impl AudioPipeline {
    pub fn new(sample_rate: f32, buffer_size: usize) -> Self {
        let max_delay_samples = (sample_rate * 2.0) as usize; // 2 секунды максимум
        
        Self {
            parameters: AudioParameters::default(),
            noise_generator: NoiseGenerator::new(),
            dsp_processor: DspProcessor::new(sample_rate, max_delay_samples),
            input_buffer: HeapRb::new(buffer_size * 4),
            output_buffer: HeapRb::new(buffer_size * 4),
            ai_input_sender: None,
            ai_output_receiver: None,
            samples_processed: 0,
            is_processing: AtomicBool::new(false),
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
        
        // Обрабатываем каждый сэмпл
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
}

/// Создает экземпляр аудиоконвейера и возвращает указатель на него.
#[no_mangle]
pub extern "C" fn create_pipeline() -> *mut c_void {
    println!("Rust: create_pipeline() вызван.");
    let pipeline = Box::new(AudioPipeline::new(44100.0, 512));
    Box::into_raw(pipeline) as *mut c_void
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
