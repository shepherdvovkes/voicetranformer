# Платформо-специфичная поддержка

DSP Core теперь поддерживает нативные аудио API:

## macOS (Core Audio + Neural Engine)
- ✅ Поддержка Apple Silicon NPU для AI эффектов
- ✅ Низкая задержка (64-256 сэмплов)
- ✅ Core ML интеграция

## Linux (ALSA)
- ✅ Прямой доступ к ALSA PCM
- ✅ Поддержка PulseAudio/JACK/PipeWire
- ✅ RT-kernel оптимизации

## Windows (WASAPI)
- 🚧 В разработке
- ✅ Базовая поддержка

## Использование

```rust
// Автоматическая инициализация лучшего API
let pipeline = AudioPipeline::new_with_platform()?;

// Проверка возможностей
println!("{}", pipeline.platform_info());
println!("NPU: {}", pipeline.supports_neural_engine());
println!("Low latency: {}", pipeline.supports_low_latency());
```