use std::sync::Once;

const DIRECTORY: &str = "sd:/ultimate/dionysus";
const OLD_DIRECTORY: &str = "sd:/ultimate/clone_engine";
const CARRIED: &[&str] = &["config.toml", "kinds.txt", "mii_parts.txt", "settings.toml"];
const PLUGINS: &str = "rom:/skyline/plugins";
const OLD_PLUGIN: &str = "libssbu_clone_engine";
const EXIT_APPLICATION: &[u8] = b"_ZN2nn2oe15ExitApplicationEv\0";

static CARRY: Once = Once::new();

fn old_plugin() -> Option<String> {
    std::fs::read_dir(PLUGINS)
        .ok()?
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .find(|name| {
            let name = name.to_ascii_lowercase();
            name.starts_with(OLD_PLUGIN) && name.ends_with(".nro")
        })
}

unsafe fn exit_game() -> ! {
    let mut address = 0usize;
    let result = skyline::nn::ro::LookupSymbol(&mut address as *mut usize, EXIT_APPLICATION.as_ptr());
    if result == 0 && address != 0 {
        let exit: extern "C" fn() = core::mem::transmute(address);
        exit();
    }
    std::process::abort();
}

pub(crate) fn stop_if_old_plugin_installed() {
    let Some(found) = old_plugin() else {
        return;
    };
    skyline::println!("[dionysus] {PLUGINS}/{found} is still installed; the two cannot run together, so the game closes");
    skyline::error::show_error(
        69,
        "Dionysus replaces libssbu_clone_engine.nro. Delete the old file.\0",
        "Dionysus is the new name of the Clone Engine plugin. Both are installed and cannot run together, so the game will close.\n\nDelete every libssbu_clone_engine file in atmosphere/contents/01006A800016E000/romfs/skyline/plugins, keep libssbu_dionysus.nro, then start the game again.\0",
    );
    unsafe { exit_game() }
}

pub(crate) fn carry_old_folder() {
    CARRY.call_once(|| {
        let mut carried = Vec::new();
        for file in CARRIED {
            let destination = format!("{DIRECTORY}/{file}");
            if std::fs::metadata(&destination).is_ok() {
                continue;
            }
            let source = format!("{OLD_DIRECTORY}/{file}");
            let Ok(bytes) = std::fs::read(&source) else {
                continue;
            };
            if let Err(error) = std::fs::create_dir_all(DIRECTORY) {
                skyline::println!("[dionysus] could not create {DIRECTORY} ({error}); {OLD_DIRECTORY} stays unread");
                return;
            }
            match std::fs::write(&destination, bytes) {
                Ok(()) => carried.push(*file),
                Err(error) => skyline::println!("[dionysus] could not copy {source} to {destination} ({error})"),
            }
        }
        if !carried.is_empty() {
            skyline::println!(
                "[dionysus] carried {} from {OLD_DIRECTORY} to {DIRECTORY}; the old folder is left as it was",
                carried.join(", ")
            );
        }
    });
}
