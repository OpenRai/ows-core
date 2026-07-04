use crate::CliError;

pub fn diag(retune: bool, json: bool) -> Result<(), CliError> {
    let diagnostics = ows_lib::nano_rpc::local_pow_diagnostics(retune);

    if json {
        let gpu = diagnostics.gpu.map(|gpu| {
            serde_json::json!({
                "backend_api": gpu.backend_api,
                "adapter_name": gpu.adapter_name,
                "driver_info": gpu.driver_info,
                "vendor_id": gpu.vendor_id,
                "device_id": gpu.device_id,
                "max_compute_workgroups_per_dimension": gpu.max_compute_workgroups_per_dimension,
                "dispatch_x": gpu.dispatch_x,
                "nonces_per_dispatch": gpu.nonces_per_dispatch,
                "tuning_source": gpu.tuning_source,
                "cache_path": gpu.cache_path,
            })
        });
        println!(
            "{}",
            serde_json::json!({
                "local_pow_backend": diagnostics.backend,
                "local_pow_recommended": diagnostics.recommended,
                "gpu": gpu,
                "gpu_error": diagnostics.gpu_error,
            })
        );
        return Ok(());
    }

    println!("local_pow_backend: {}", diagnostics.backend);
    println!("local_pow_recommended: {}", diagnostics.recommended);
    if let Some(gpu_error) = diagnostics.gpu_error.as_deref() {
        println!("gpu_error: {gpu_error}");
    }
    if let Some(gpu) = diagnostics.gpu {
        println!("backend_api: {}", gpu.backend_api);
        println!("adapter_name: {}", gpu.adapter_name);
        println!("driver_info: {}", gpu.driver_info);
        println!("vendor_id: {}", gpu.vendor_id);
        println!("device_id: {}", gpu.device_id);
        println!(
            "max_compute_workgroups_per_dimension: {}",
            gpu.max_compute_workgroups_per_dimension
        );
        println!("dispatch_x: {}", gpu.dispatch_x);
        println!("nonces_per_dispatch: {}", gpu.nonces_per_dispatch);
        println!("tuning_source: {}", gpu.tuning_source);
        if let Some(cache_path) = gpu.cache_path {
            println!("cache_path: {cache_path}");
        }
    }

    Ok(())
}
