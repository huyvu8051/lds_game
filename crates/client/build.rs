//! Fills in what Valve's iOS build of Steam Audio leaves out.
//!
//! `libphonon.a` for iOS is compiled with its FFT and its SOFA reader left
//! unresolved -- ten symbols, `pffft_*` and `mysofa_*` -- where every other
//! platform ships them inside the library. The desktop dylib has them baked
//! in, so this is an iOS-only hole, and without it the app does not link at
//! all.
//!
//! Both are small permissive C libraries, vendored under `vendor/` rather
//! than fetched, so a build needs no network and the bytes are the ones that
//! were tested:
//!   - PFFFT, Julien Pommier, bitbucket.org/jpommier/pffft (FFTPACK licence)
//!   - libmysofa 1.3.5, github.com/hoene/libmysofa (BSD-3-Clause)
//! `config.h` and `mysofa_export.h` are what libmysofa's CMake would have
//! generated; they are written out by hand because there is no cmake here.
fn main() {
    println!("cargo:rerun-if-changed=vendor");
    // The simulator has no Steam Audio to complete, so it needs none of this.
    let ios = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("ios");
    let simulator = std::env::var("CARGO_CFG_TARGET_ABI").as_deref() == Ok("sim");
    if !ios || simulator {
        return;
    }

    // Warnings off for both: this is other people's C, held at the revision
    // it was tested at, and libmysofa alone has thirty sign-compares that
    // nobody here is going to fix.
    cc::Build::new()
        .file("vendor/pffft/pffft.c")
        .opt_level(2)
        .warnings(false)
        .compile("pffft");

    let mut mysofa = cc::Build::new();
    mysofa.include("vendor/libmysofa").opt_level(2).warnings(false);
    for dir in ["hrtf", "hdf", "resampler"] {
        mysofa.include(format!("vendor/libmysofa/{dir}"));
        // Sorted: read_dir's order is the filesystem's, and an archive whose
        // member order moves between builds invalidates caches for nothing.
        let mut sources: Vec<_> = std::fs::read_dir(format!("vendor/libmysofa/{dir}"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|e| e == "c"))
            .collect();
        sources.sort();
        mysofa.files(sources);
    }
    mysofa.compile("mysofa");

    // libmysofa reads gzipped SOFA files; the iOS SDK carries zlib.
    println!("cargo:rustc-link-lib=z");
    // libphonon is C++ and nobody else asks for the runtime: on the desktop
    // the dylib carries its own linkage, so the static iOS library is the
    // only place the C++ personality and the __cxa_* calls come up bare.
    println!("cargo:rustc-link-lib=c++");
}
