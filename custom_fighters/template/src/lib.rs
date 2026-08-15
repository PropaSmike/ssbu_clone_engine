#![feature(proc_macro_hygiene)]
#![allow(
    non_snake_case,
    non_upper_case_globals,
    unused_imports,
    unused_variables,
    clippy::borrow_interior_mutable_const
)]

pub use smashline::skyline_smash as smash;

mod acmd;
mod articles;
mod callbacks;
mod csk;
#[cfg(feature = "kirby_copy_example")]
mod kirby_copy;
mod params;
#[cfg(feature = "shared_hook_example")]
mod shared_hook;
mod status;

pub const BASE_FIGHTER_KIND: i32 = 0;
pub const BASE_RESOURCE_NAME: &str = "mario";
pub const RESOURCE_NAME: &str = "template_fighter";
pub const UI_CHARA_NAME: &str = "ui_chara_template_fighter";
pub const FIGHTER_KIND_NAME: &str = "fighter_kind_template_fighter";
pub const COLOR_START: u32 = 0;
pub const COLOR_COUNT: u32 = 8;

static CUSTOM_KIND: clone_engine_api::CloneKind =
    clone_engine_api::CloneKind::new(FIGHTER_KIND_NAME);

pub fn kind() -> Option<i32> {
    CUSTOM_KIND.get()
}

fn register_descriptor() -> Result<i32, clone_engine_api::Error> {
    let mut descriptor = clone_engine_api::CloneRegistration::new(
        clone_engine_api::KIND_AUTO,
        BASE_FIGHTER_KIND,
        UI_CHARA_NAME,
        FIGHTER_KIND_NAME,
        RESOURCE_NAME,
        BASE_RESOURCE_NAME,
    );
    descriptor.color_start = COLOR_START;
    descriptor.color_count = COLOR_COUNT;

    descriptor.effect_namespace = 0;
    descriptor.article_namespace = 0;

    #[cfg(feature = "kirby_copy_example")]
    {
        descriptor.copy_status_first = kirby_copy::STATUS_FIRST;
        descriptor.copy_status_count = kirby_copy::STATUS_COUNT;
    }

    clone_engine_api::allocate(&descriptor)
}

extern "C" fn mods_mounted(_event: arcropolis_api::Event) {
    if kind().is_none() {
        clone_engine_api::elog!("[template] mount event ignored: no allocated kind");
        return;
    }
    csk::install();
}

fn register_mount_callback() {
    unsafe {
        extern "C" {
            fn arcrop_register_event_callback(
                ty: arcropolis_api::Event,
                callback: arcropolis_api::EventCallbackFn,
            );
        }
        arcrop_register_event_callback(arcropolis_api::Event::ModFilesystemMounted, mods_mounted);
    }
}

#[skyline::main(name = "clone_engine_moveset_template")]
pub fn main() {
    let required = clone_engine_api::CAP_FIGHTER_IDENTITY
        | clone_engine_api::CAP_SMASHLINE_BRIDGE
        | clone_engine_api::CAP_FIGHTER_ARTICLES
        | clone_engine_api::CAP_PARAMCONFIG_BRIDGE
        | if cfg!(feature = "kirby_copy_example") {
            clone_engine_api::CAP_KIRBY_COPY
        } else {
            0
        }
        | if cfg!(feature = "shared_hook_example") {
            clone_engine_api::CAP_SHARED_HOOKS
        } else {
            0
        };
    let compiled = clone_engine_api::compiled_capabilities();
    let runtime = clone_engine_api::runtime_capabilities();
    if compiled & required != required {
        skyline::println!(
            "[template] disabled: missing compiled capabilities {:#x}",
            required & !compiled
        );
        return;
    }
    if runtime & required != required {
        skyline::println!(
            "[template] disabled: runtime preflight failed for {:#x}",
            required & !runtime
        );
        return;
    }
    let kind = match register_descriptor() {
        Ok(kind) => kind,
        Err(error) => {
            skyline::println!("[template] Clone Engine registration failed: {error:?}");
            return;
        }
    };
    CUSTOM_KIND.store(kind);

    acmd::install();
    status::install();
    callbacks::install();
    params::install(kind);
    articles::install(kind);

    #[cfg(feature = "kirby_copy_example")]
    kirby_copy::install(kind);
    #[cfg(feature = "shared_hook_example")]
    shared_hook::install();

    register_mount_callback();
    clone_engine_api::elog!(
        "[template] registered identity={} kind={} base={} max={} backend={:#x}",
        FIGHTER_KIND_NAME,
        kind,
        clone_engine_api::base_kind(kind),
        clone_engine_api::max_custom_kind(),
        clone_engine_api::native_backend_status()
    );
}
