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
