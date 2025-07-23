#!/bin/bash

echo "🎵 Запуск Voice Transformer Demo с NPU поддержкой..."
echo ""

# Проверяем операционную систему и архитектуру
OS=$(uname -s)
ARCH=$(uname -m)

echo "📊 Информация о системе:"
echo "  • Платформа: $OS ($ARCH)"

if [[ "$OS" == "Darwin" && "$ARCH" == "arm64" ]]; then
    echo "  • NPU поддержка: ✅ Apple Silicon Neural Engine"
    echo "  • Оптимизация: Core ML + Core Audio"
else
    echo "  • NPU поддержка: ❌ CPU fallback режим"
    echo "  • Аудио система: ALSA (Linux) / WASAPI (Windows)"
fi

echo ""
echo "🎯 Демонстрационные эффекты:"
echo "  • Голос-Чейнжер: DSP → NPU → Post-processing"
echo "  • AI эффекты: Робот, Демон, Пришелец"
echo "  • DSP эффекты: Монстр, Пещера, Рация и др."
echo ""

# Проверяем доступность порта
PORT=8080
while lsof -Pi :$PORT -sTCP:LISTEN -t >/dev/null 2>&1; do
    PORT=$((PORT+1))
done

echo "🌐 Запуск веб-сервера на порту $PORT..."

# Проверяем наличие библиотеки
RUST_LIB=""
if [[ -f "dsp_core/target/release/libdsp_core.so" ]]; then
    RUST_LIB="libdsp_core.so"
elif [[ -f "dsp_core/target/release/libdsp_core.dylib" ]]; then
    RUST_LIB="libdsp_core.dylib"
elif [[ -f "dsp_core/target/release/libdsp_core.dll" ]]; then
    RUST_LIB="libdsp_core.dll"
fi

if [[ -n "$RUST_LIB" ]]; then
    echo "✅ Rust DSP ядро: $RUST_LIB"
else
    echo "⚠️  Rust DSP ядро не найдено (пересоберите: cd dsp_core && cargo build --release)"
fi

echo ""
echo "🚀 Открывайте http://localhost:$PORT в браузере"
echo ""
echo "💡 Инструкции:"
echo "  1. Нажмите 'Initialize Audio' в интерфейсе"
echo "  2. Разрешите доступ к микрофону"
echo "  3. Выберите 'Голос-Чейнжер' для полной демонстрации"
echo "  4. Наблюдайте за нагрузкой CPU/GPU/NPU в реальном времени"
echo ""

# Запускаем простой веб-сервер
if command -v python3 &> /dev/null; then
    echo "🔧 Используется Python 3 веб-сервер"
    cd frontend && python3 -m http.server $PORT
elif command -v python &> /dev/null; then
    echo "🔧 Используется Python 2 веб-сервер"
    cd frontend && python -m SimpleHTTPServer $PORT
elif command -v node &> /dev/null && command -v npx &> /dev/null; then
    echo "🔧 Используется Node.js веб-сервер"
    cd frontend && npx http-server -p $PORT
else
    echo "❌ Не найден подходящий веб-сервер."
    echo ""
    echo "Установите один из:"
    echo "  • Python: apt install python3 (Linux) / brew install python (macOS)"
    echo "  • Node.js: https://nodejs.org"
    echo ""
    echo "💡 Альтернатива: откройте frontend/index.html прямо в браузере"
    exit 1
fi