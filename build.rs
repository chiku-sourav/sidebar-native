fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=build.rs");

    // Only embed resources on Windows targets
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");

        // Stamp metadata visible in Windows Explorer -> Properties -> Details
        res.set(
            "FileDescription",
            "SideVitals — Windows 11 Diagnostics Flyout",
        );
        res.set("ProductName", "SideVitals");
        res.set("OriginalFilename", "sidevitals.exe");
        res.set("LegalCopyright", "GPL-3.0 License");

        // For the windows-gnu toolchain, winres uses `windres` + `gcc`.
        // Neither lives in the standard PATH on a typical Windows machine, so
        // we locate the MSYS2 ucrt64 (or mingw64) environment and temporarily
        // prepend it to PATH so both tools are found automatically.
        let msys2_dirs = [r"C:\msys64\ucrt64\bin", r"C:\msys64\mingw64\bin"];
        for dir in &msys2_dirs {
            if std::path::Path::new(&format!("{}\\windres.exe", dir)).exists() {
                // Prepend to PATH for this build-script process
                let current_path = std::env::var("PATH").unwrap_or_default();
                std::env::set_var("PATH", format!("{};{}", dir, current_path));
                res.set_windres_path(&format!("{}\\windres.exe", dir));
                break;
            }
        }

        res.compile().expect("Failed to compile Windows resources");

        let out_dir = std::env::var("OUT_DIR").unwrap_or_default();
        let res_o = format!("{}/resource.o", out_dir);
        let res_res = format!("{}/resource.res", out_dir);
        if std::path::Path::new(&res_o).exists() {
            println!("cargo:rustc-link-arg-bins={}", res_o);
        } else if std::path::Path::new(&res_res).exists() {
            println!("cargo:rustc-link-arg-bins={}", res_res);
        }
    }
}
