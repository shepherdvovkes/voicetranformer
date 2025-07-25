#!/bin/bash

# setup_subdirs.sh
# Этот скрипт создает структуру каталогов и файлы для ядра на Rust
# и фронтенда ВНУТРИ существующего проекта.
# Запускать из корневой папки проекта!

echo "--- Создание подкаталогов и файлов проекта ---"

# --- 1. Создание структуры для ядра на Rust ---
echo "-> Создание структуры для Rust Core..."

# Создаем каталог для исходного кода ядра
mkdir -p dsp_core/src

# Создаем Cargo.toml для Rust-ядра
cat <<EOF > dsp_core/Cargo.toml
[package]
name = "dsp_core"
version = "0.1.0"
edition = "2021"

# Для создания нативной библиотеки, которую можно вызывать из других языков
[lib]
crate-type = ["cdylib", "staticlib"]

[dependencies]
# Для кросс-платформенного аудио ввода/вывода
cpal = "0.15"

# Для качественного сдвига высоты тона и формант
signalsmith-stretch = "0.1.1"

# Для работы с WAV файлами (полезно для отладки)
hound = "3.5.1"

# Для удобной обработки ошибок
anyhow = "1.0"

# Для безблокировочной коммуникации между потоками
ringbuf = "0.3"
crossbeam-channel = "0.5"

# Для безопасного управления параметрами в реальном времени
atomic_float = "0.1"
EOF

# Создаем файл src/lib.rs с базовой структурой для FFI
# Это основа для вызова Rust-кода из нативного приложения (Swift, Tauri)
cat <<EOF > dsp_core/src/lib.rs
use std::ffi::c_void;

// TODO: Определить структуру для аудиоконвейера
pub struct AudioPipeline {
    // ... поля для процессоров, буферов и т.д.
}

/// Создает экземпляр аудиоконвейера и возвращает указатель на него.
#[no_mangle]
pub extern "C" fn create_pipeline() -> *mut c_void {
    println!("Rust: create_pipeline() вызван.");
    let pipeline = Box::new(AudioPipeline {});
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
    let _pipeline = &mut *(pipeline_ptr as *mut AudioPipeline);
    let _input_slice = std::slice::from_raw_parts(input, len);
    let output_slice = std::slice::from_raw_parts_mut(output, len);

    // TODO: Здесь будет логика вызова DSP-процессоров.
    // Пока просто заполняем тишиной.
    for sample in output_slice.iter_mut() {
        *sample = 0.0;
    }
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
EOF

echo "Rust Core структура создана в каталоге 'dsp_core/'."

# --- 2. Создание структуры для Фронтенда (UI Прототип) ---
echo "-> Создание структуры для Frontend..."

mkdir -p frontend

# Создаем HTML-файл с профессиональным интерфейсом
cat <<'EOF' > frontend/index.html
<!DOCTYPE html>
<html lang="ru">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Voice Transformer UI - Professional</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <script src="https://cdnjs.cloudflare.com/ajax/libs/tone/14.7.77/Tone.js"></script>
    <script src="https://unpkg.com/lucide@latest"></script>
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap" rel="stylesheet">
    <style>
        body {
            font-family: 'Inter', sans-serif;
            background-color: #121212;
            color: #E5E7EB; /* text-gray-200 */
        }
        .panel {
            background-color: #1E1E1E;
            border: 1px solid #374151; /* border-gray-700 */
            border-radius: 0.75rem;
        }
        .preset-card {
            background-color: #374151; /* bg-gray-700 */
            border: 1px solid #4B5563; /* border-gray-600 */
            transition: background-color 0.2s ease, border-color 0.2s ease;
        }
        .preset-card:hover {
            background-color: #4B5563; /* bg-gray-600 */
            border-color: #6B7280; /* border-gray-500 */
        }
        .preset-card.active {
            background-color: #1f2937;
            border-color: #F59E0B; /* border-amber-500 */
            box-shadow: 0 0 0 2px rgba(245, 158, 11, 0.4);
        }
        .preset-card.active svg {
            color: #F59E0B; /* text-amber-500 */
        }
        input[type="range"] {
            -webkit-appearance: none;
            appearance: none;
            width: 100%;
            height: 4px;
            background: #4B5563; /* bg-gray-600 */
            border-radius: 2px;
            outline: none;
        }
        input[type="range"]::-webkit-slider-thumb {
            -webkit-appearance: none;
            appearance: none;
            width: 18px;
            height: 18px;
            background: #E5E7EB; /* bg-gray-200 */
            cursor: pointer;
            border-radius: 50%;
            border: 2px solid #1E1E1E;
            transition: background-color 0.2s ease;
        }
        input[type="range"]::-webkit-slider-thumb:hover {
            background: #F59E0B;
        }
        .toggle-btn {
            background-color: #4B5563;
            transition: background-color 0.2s ease;
        }
        .toggle-btn.active {
            background-color: #F59E0B;
        }
        .toggle-btn.active svg {
            color: #121212;
        }
        .progress-bar {
            transition: width 0.5s ease-in-out;
        }
    </style>
</head>
<body>

    <div class="container mx-auto p-4 max-w-7xl">
        <!-- Заголовок -->
        <header class="text-center my-6 md:my-10">
            <h1 class="text-3xl md:text-4xl font-bold text-white tracking-tight">VOICE TRANSFORMER</h1>
            <p class="text-md text-gray-400 mt-1">Real-time Voice Processing Core</p>
        </header>

        <!-- Основной контейнер -->
        <main class="grid grid-cols-1 lg:grid-cols-3 gap-6">

            <!-- Левая колонка: Управление -->
            <div class="lg:col-span-1 space-y-6">
                
                <!-- Кнопка включения -->
                <div class="panel p-5">
                    <button id="start-btn" class="w-full flex items-center justify-center gap-3 bg-amber-600 hover:bg-amber-700 text-white font-bold py-3 px-4 rounded-lg text-lg transition duration-300">
                        <i data-lucide="mic"></i>
                        <span>Включить микрофон</span>
                    </button>
                    <p id="status" class="text-center text-gray-500 mt-3 text-sm">Требуется доступ к микрофону для начала работы</p>
                </div>

                <!-- Пресеты -->
                <div class="panel p-5">
                    <h2 class="text-xl font-bold mb-4 text-gray-200">Пресеты</h2>
                    <div id="presets-grid" class="grid grid-cols-3 gap-3">
                        <!-- Пресеты будут добавлены сюда с помощью JS -->
                    </div>
                </div>

                <!-- Фоновые шумы -->
                <div class="panel p-5">
                    <h2 class="text-xl font-bold mb-4 text-gray-200">Фоновые шумы</h2>
                    <div id="noise-controls" class="space-y-5">
                        <!-- Элементы управления шумом будут добавлены сюда с помощью JS -->
                    </div>
                </div>
            </div>

            <!-- Правая колонка: Визуализация и Нагрузка -->
            <div class="lg:col-span-2 space-y-6">
                <!-- Нагрузка на систему -->
                <div class="panel p-5">
                    <h2 class="text-xl font-bold mb-4 text-gray-200">Нагрузка на систему (Симуляция)</h2>
                    <div id="system-load" class="space-y-4">
                        <!-- CPU -->
                        <div>
                            <div class="flex justify-between items-center mb-1">
                                <span class="text-sm font-medium text-gray-400">CPU</span>
                                <span id="cpu-load-text" class="text-sm font-medium text-gray-300">0%</span>
                            </div>
                            <div class="w-full bg-gray-700 rounded-full h-2.5">
                                <div id="cpu-load-bar" class="bg-blue-500 h-2.5 rounded-full progress-bar" style="width: 0%"></div>
                            </div>
                        </div>
                        <!-- GPU -->
                        <div>
                            <div class="flex justify-between items-center mb-1">
                                <span class="text-sm font-medium text-gray-400">GPU</span>
                                <span id="gpu-load-text" class="text-sm font-medium text-gray-300">0%</span>
                            </div>
                            <div class="w-full bg-gray-700 rounded-full h-2.5">
                                <div id="gpu-load-bar" class="bg-green-500 h-2.5 rounded-full progress-bar" style="width: 0%"></div>
                            </div>
                        </div>
                        <!-- NPU -->
                        <div>
                            <div class="flex justify-between items-center mb-1">
                                <span class="text-sm font-medium text-gray-400">NPU (Neural Engine)</span>
                                <span id="npu-load-text" class="text-sm font-medium text-gray-300">0%</span>
                            </div>
                            <div class="w-full bg-gray-700 rounded-full h-2.5">
                                <div id="npu-load-bar" class="bg-amber-500 h-2.5 rounded-full progress-bar" style="width: 0%"></div>
                            </div>
                        </div>
                    </div>
                </div>

                <!-- Визуализация сигнала -->
                <div class="panel p-5">
                    <h2 class="text-xl font-bold mb-4 text-gray-200">Визуализация сигнала</h2>
                    <div class="space-y-6">
                        <div>
                            <h3 class="text-md font-semibold mb-2 text-gray-400">Осциллограф (Форма волны)</h3>
                            <canvas id="oscilloscope" class="w-full h-48 bg-black rounded-md"></canvas>
                        </div>
                        <div>
                            <h3 class="text-md font-semibold mb-2 text-gray-400">Анализатор спектра (Частоты)</h3>
                            <canvas id="spectrum" class="w-full h-48 bg-black rounded-md"></canvas>
                        </div>
                    </div>
                </div>
            </div>
        </main>
    </div>

    <script>
        // --- Глобальные переменные ---
        let audioContext;
        let analyser;
        let microphone;
        let isInitialized = false;
        const noiseGenerators = {};
        let currentLoad = { cpu: 0, gpu: 0, npu: 0 };
        let baseLoad = { cpu: 4, gpu: 8, npu: 1 };

        // --- Элементы DOM ---
        const startBtn = document.getElementById('start-btn');
        const statusEl = document.getElementById('status');
        const presetsGrid = document.getElementById('presets-grid');
        const noiseControlsContainer = document.getElementById('noise-controls');
        const oscCanvas = document.getElementById('oscilloscope');
        const oscCtx = oscCanvas.getContext('2d');
        const specCanvas = document.getElementById('spectrum');
        const specCtx = specCanvas.getContext('2d');
        
        const cpuLoadBar = document.getElementById('cpu-load-bar');
        const gpuLoadBar = document.getElementById('gpu-load-bar');
        const npuLoadBar = document.getElementById('npu-load-bar');
        const cpuLoadText = document.getElementById('cpu-load-text');
        const gpuLoadText = document.getElementById('gpu-load-text');
        const npuLoadText = document.getElementById('npu-load-text');

        // --- Данные для UI ---
        const presets = [
            { name: 'Робот', icon: 'bot', type: 'ai' },
            { name: 'Монстр', icon: 'skull', type: 'dsp' },
            { name: 'Высокий', icon: 'arrow-up-circle', type: 'dsp' },
            { name: 'Пещера', icon: 'mountain', type: 'dsp' },
            { name: 'Рация', icon: 'radio-tower', type: 'dsp' },
            { name: 'Демон', icon: 'flame', type: 'ai' },
            { name: 'Пришелец', icon: 'alien', type: 'ai' },
            { name: 'Собор', icon: 'church', type: 'dsp' },
            { name: 'Под водой', icon: 'waves', type: 'dsp' },
        ];

        const noiseTypes = [
            { id: 'white', name: 'Белый шум (Статика)' },
            { id: 'pink', name: 'Розовый шум (Дождь)' },
            { id: 'brown', name: 'Коричневый шум (Ветер)' },
        ];

        // --- Инициализация UI ---
        function initializeUI() {
            presets.forEach(preset => {
                const button = document.createElement('button');
                button.className = 'preset-card p-2 rounded-lg flex flex-col items-center justify-center aspect-square focus:outline-none';
                button.innerHTML = `
                    <i data-lucide="${preset.icon}" class="w-8 h-8 text-gray-300"></i>
                    <span class="text-xs mt-2 text-center text-gray-400">${preset.name}</span>`;
                button.addEventListener('click', () => {
                    if (!isInitialized) return;
                    document.querySelectorAll('.preset-card').forEach(p => p.classList.remove('active'));
                    button.classList.add('active');
                    console.log(`Пресет "${preset.name}" (${preset.type}) активирован.`);
                    
                    // Симуляция нагрузки
                    if (preset.type === 'ai') {
                        // Высокая нагрузка на NPU, минимальная на CPU/GPU
                        currentLoad = { cpu: baseLoad.cpu + 4, gpu: baseLoad.gpu + 2, npu: 25 };
                    } else {
                        // Повышенная нагрузка на CPU для DSP, NPU в простое
                        currentLoad = { cpu: baseLoad.cpu + 15, gpu: baseLoad.gpu, npu: baseLoad.npu };
                    }
                });
                presetsGrid.appendChild(button);
            });

            noiseTypes.forEach(noise => {
                const controlDiv = document.createElement('div');
                controlDiv.innerHTML = `
                    <div class="flex items-center justify-between">
                        <label for="${noise.id}-toggle" class="font-medium text-gray-300">${noise.name}</label>
                        <button id="${noise.id}-toggle" class="toggle-btn p-2 rounded-full focus:outline-none" data-state="off">
                            <i data-lucide="play" class="w-4 h-4 text-gray-200"></i>
                        </button>
                    </div>
                    <input type="range" id="${noise.id}-volume" min="-40" max="0" value="-20" step="1" class="w-full mt-2" disabled>
                `;
                noiseControlsContainer.appendChild(controlDiv);

                const toggleBtn = document.getElementById(`${noise.id}-toggle`);
                const volumeSlider = document.getElementById(`${noise.id}-volume`);

                toggleBtn.addEventListener('click', () => {
                    if (!isInitialized) return;
                    const noiseGen = noiseGenerators[noise.id];
                    if (toggleBtn.dataset.state === 'off') {
                        noiseGen.start();
                        toggleBtn.dataset.state = 'on';
                        toggleBtn.innerHTML = '<i data-lucide="pause" class="w-4 h-4"></i>';
                        toggleBtn.classList.add('active');
                        volumeSlider.disabled = false;
                    } else {
                        noiseGen.stop();
                        toggleBtn.dataset.state = 'off';
                        toggleBtn.innerHTML = '<i data-lucide="play" class="w-4 h-4"></i>';
                        toggleBtn.classList.remove('active');
                        volumeSlider.disabled = true;
                    }
                    lucide.createIcons();
                });

                volumeSlider.addEventListener('input', (e) => {
                    if (!isInitialized) return;
                    noiseGenerators[noise.id].volume.value = e.target.value;
                });
            });
            lucide.createIcons();
        }

        // --- Основная аудио логика ---
        async function startAudio() {
            if (isInitialized) return;
            try {
                await Tone.start();
                audioContext = Tone.getContext().rawContext;
                const stream = await navigator.mediaDevices.getUserMedia({ audio: true, video: false });
                microphone = audioContext.createMediaStreamSource(stream);
                analyser = audioContext.createAnalyser();
                analyser.fftSize = 2048;
                
                noiseTypes.forEach(noise => {
                    noiseGenerators[noise.id] = new Tone.Noise(noise.id).toDestination();
                    noiseGenerators[noise.id].volume.value = -20;
                });

                microphone.connect(analyser);
                analyser.connect(audioContext.destination);

                isInitialized = true;
                currentLoad = baseLoad; // Устанавливаем базовую нагрузку
                startBtn.innerHTML = '<i data-lucide="check-circle"></i><span>Микрофон активен</span>';
                startBtn.classList.replace('bg-amber-600', 'bg-green-600');
                startBtn.classList.remove('hover:bg-amber-700');
                startBtn.disabled = true;
                statusEl.textContent = 'Аудиосистема инициализирована';
                lucide.createIcons();

                resizeCanvases();
                draw();
                setInterval(updateLoadUI, 500); // Запускаем обновление UI нагрузки
            } catch (err) {
                console.error('Ошибка при доступе к микрофону:', err);
                statusEl.textContent = 'Ошибка: Не удалось получить доступ к микрофону.';
                statusEl.classList.add('text-red-400');
            }
        }
        
        // --- Логика нагрузки и отрисовки ---
        function updateLoadUI() {
            if (!isInitialized) return;
            
            // Добавляем небольшие флуктуации для реалистичности
            const fluctuate = (val) => Math.max(0, Math.min(100, val + (Math.random() * 4 - 2)));
            
            const cpu = fluctuate(currentLoad.cpu);
            const gpu = fluctuate(currentLoad.gpu);
            const npu = fluctuate(currentLoad.npu);

            cpuLoadBar.style.width = `${cpu}%`;
            cpuLoadText.textContent = `${Math.round(cpu)}%`;
            gpuLoadBar.style.width = `${gpu}%`;
            gpuLoadText.textContent = `${Math.round(gpu)}%`;
            npuLoadBar.style.width = `${npu}%`;
            npuLoadText.textContent = `${Math.round(npu)}%`;
        }

        function draw() {
            if (!isInitialized) return;
            requestAnimationFrame(draw);
            drawOscilloscope();
            drawSpectrum();
        }

        function drawOscilloscope() {
            const bufferLength = analyser.fftSize;
            const dataArray = new Uint8Array(bufferLength);
            analyser.getByteTimeDomainData(dataArray);
            oscCtx.fillStyle = '#000000';
            oscCtx.fillRect(0, 0, oscCanvas.width, oscCanvas.height);
            oscCtx.lineWidth = 2;
            oscCtx.strokeStyle = '#F59E0B'; // amber-500
            oscCtx.beginPath();
            const sliceWidth = oscCanvas.width * 1.0 / bufferLength;
            let x = 0;
            for (let i = 0; i < bufferLength; i++) {
                const v = dataArray[i] / 128.0;
                const y = v * oscCanvas.height / 2;
                if (i === 0) { oscCtx.moveTo(x, y); } else { oscCtx.lineTo(x, y); }
                x += sliceWidth;
            }
            oscCtx.lineTo(oscCanvas.width, oscCanvas.height / 2);
            oscCtx.stroke();
        }

        function drawSpectrum() {
            const bufferLength = analyser.frequencyBinCount;
            const dataArray = new Uint8Array(bufferLength);
            analyser.getByteFrequencyData(dataArray);
            specCtx.fillStyle = '#000000';
            specCtx.fillRect(0, 0, specCanvas.width, specCanvas.height);
            const barWidth = (specCanvas.width / bufferLength) * 2.5;
            let barHeight;
            let x = 0;
            for (let i = 0; i < bufferLength; i++) {
                barHeight = dataArray[i];
                const percent = barHeight / 255;
                const hue = 240 - (percent * 180); // от синего (240) к красному (60)
                specCtx.fillStyle = `hsl(${hue}, 100%, 50%)`;
                specCtx.fillRect(x, specCanvas.height - barHeight / 1.5, barWidth, barHeight / 1.5);
                x += barWidth + 1;
            }
        }
        
        function resizeCanvases() {
            [oscCanvas, specCanvas].forEach(canvas => {
                canvas.width = canvas.clientWidth * window.devicePixelRatio;
                canvas.height = canvas.clientHeight * window.devicePixelRatio;
            });
        }

        // --- Event Listeners ---
        startBtn.addEventListener('click', startAudio);
        window.addEventListener('resize', resizeCanvases);

        // --- Инициализация ---
        initializeUI();
    </script>
</body>
</html>
EOF

echo "Frontend UI прототип создан в каталоге 'frontend/'."
echo ""
echo "--- Структура проекта успешно создана! ---"
echo "Для запуска Rust-ядра (в будущем):"
echo "cd dsp_core && cargo build"
echo ""
echo "Для просмотра UI прототипа:"
echo "Откройте файл 'frontend/index.html' в вашем браузере."

