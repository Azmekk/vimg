use anyhow::Result;

#[cfg(windows)]
pub fn enable() -> Result<()> {
    windows_impl::enable()
}

#[cfg(windows)]
pub fn disable() -> Result<()> {
    windows_impl::disable()
}

#[cfg(not(windows))]
pub fn enable() -> Result<()> {
    println!("Context-menu integration is Windows only.");
    Ok(())
}

#[cfg(not(windows))]
pub fn disable() -> Result<()> {
    println!("Context-menu integration is Windows only.");
    Ok(())
}

#[cfg(windows)]
mod windows_impl {
    use std::env;

    use anyhow::{Context, Result, anyhow};
    use windows_registry::CURRENT_USER;

    const SHARED_MENU_KEY: &str = r"Software\Classes\vimg.Menu";
    const SHARED_BATCH_KEY: &str = r"Software\Classes\vimg.BatchMenu";
    const EXTENSIONS: &[&str] = &[
        "png", "jpg", "jpeg", "webp", "avif", "gif", "bmp", "tif", "tiff",
    ];

    fn exe_path() -> Result<String> {
        let exe = env::current_exe().context("resolving current_exe")?;
        Ok(exe.to_string_lossy().into_owned())
    }

    pub fn enable() -> Result<()> {
        let exe = exe_path()?;
        write_shared_menus(&exe)?;
        for ext in EXTENSIONS {
            write_extension_pointer(ext)?;
        }
        println!(
            "vimg context menu installed for {} extension(s).",
            EXTENSIONS.len()
        );
        println!("On Windows 11, items appear under \"Show more options\" (shift-right-click).");
        Ok(())
    }

    pub fn disable() -> Result<()> {
        for ext in EXTENSIONS {
            let key_path = format!(r"Software\Classes\SystemFileAssociations\.{ext}\shell\vimg");
            let _ = CURRENT_USER.remove_tree(&key_path);
        }
        let _ = CURRENT_USER.remove_tree(SHARED_MENU_KEY);
        let _ = CURRENT_USER.remove_tree(SHARED_BATCH_KEY);
        println!("vimg context menu removed.");
        Ok(())
    }

    fn write_shared_menus(exe: &str) -> Result<()> {
        // Top-level convert-and-write-sibling verbs.
        write_verb(
            &format!(r"{SHARED_MENU_KEY}\shell\01-png"),
            "Convert to PNG",
            &format!("\"{exe}\" \"%1\" -f png"),
            None,
            true,
        )?;
        write_verb(
            &format!(r"{SHARED_MENU_KEY}\shell\02-jpg"),
            "Convert to JPG",
            &format!("\"{exe}\" \"%1\" -f jpg"),
            None,
            true,
        )?;
        write_verb(
            &format!(r"{SHARED_MENU_KEY}\shell\03-webp"),
            "Convert to WebP",
            &format!("\"{exe}\" \"%1\" -f webp"),
            None,
            true,
        )?;
        write_verb(
            &format!(r"{SHARED_MENU_KEY}\shell\04-avif"),
            "Convert to AVIF",
            &format!("\"{exe}\" \"%1\" -f avif"),
            None,
            true,
        )?;
        // Nested submenu: "Convert to a folder" → writes outputs into
        // <source-folder>_optimized\. CommandFlags=0x40 puts a separator above it.
        let folder_key = CURRENT_USER
            .create(format!(r"{SHARED_MENU_KEY}\shell\05-folder"))
            .map_err(|e| anyhow!("creating 05-folder: {e}"))?;
        folder_key
            .set_string("MUIVerb", "Convert to a folder")
            .map_err(|e| anyhow!("setting MUIVerb on 05-folder: {e}"))?;
        folder_key
            .set_string("ExtendedSubCommandsKey", "vimg.BatchMenu")
            .map_err(|e| anyhow!("setting ExtendedSubCommandsKey on 05-folder: {e}"))?;
        folder_key
            .set_u32("CommandFlags", 0x40)
            .map_err(|e| anyhow!("setting CommandFlags on 05-folder: {e}"))?;
        // "Optimize" — bare in-place verb, no separator (folder entry already has one).
        write_verb(
            &format!(r"{SHARED_MENU_KEY}\shell\06-opt"),
            "Optimize",
            &format!("\"{exe}\" \"%1\""),
            None,
            true,
        )?;

        // Batch submenu — each verb gets MultiSelectModel=Player so multi-selection
        // launches a single vimg process with all selected files as args.
        write_verb(
            &format!(r"{SHARED_BATCH_KEY}\shell\01-png"),
            "PNG",
            &format!("\"{exe}\" --to-folder -f png \"%1\""),
            None,
            true,
        )?;
        write_verb(
            &format!(r"{SHARED_BATCH_KEY}\shell\02-jpg"),
            "JPG",
            &format!("\"{exe}\" --to-folder -f jpg \"%1\""),
            None,
            true,
        )?;
        write_verb(
            &format!(r"{SHARED_BATCH_KEY}\shell\03-webp"),
            "WebP",
            &format!("\"{exe}\" --to-folder -f webp \"%1\""),
            None,
            true,
        )?;
        write_verb(
            &format!(r"{SHARED_BATCH_KEY}\shell\04-avif"),
            "AVIF",
            &format!("\"{exe}\" --to-folder -f avif \"%1\""),
            None,
            true,
        )?;
        Ok(())
    }

    fn write_verb(
        verb_path: &str,
        label: &str,
        command: &str,
        command_flags: Option<u32>,
        multi_select_player: bool,
    ) -> Result<()> {
        let key = CURRENT_USER
            .create(verb_path)
            .map_err(|e| anyhow!("creating {verb_path}: {e}"))?;
        key.set_string("MUIVerb", label)
            .map_err(|e| anyhow!("setting MUIVerb on {verb_path}: {e}"))?;
        if let Some(flags) = command_flags {
            key.set_u32("CommandFlags", flags)
                .map_err(|e| anyhow!("setting CommandFlags on {verb_path}: {e}"))?;
        }
        if multi_select_player {
            key.set_string("MultiSelectModel", "Player")
                .map_err(|e| anyhow!("setting MultiSelectModel on {verb_path}: {e}"))?;
        }
        write_command(&format!(r"{verb_path}\command"), command)?;
        Ok(())
    }

    fn write_extension_pointer(ext: &str) -> Result<()> {
        let key_path = format!(r"Software\Classes\SystemFileAssociations\.{ext}\shell\vimg");
        let key = CURRENT_USER
            .create(&key_path)
            .map_err(|e| anyhow!("creating {key_path}: {e}"))?;
        key.set_string("MUIVerb", "Convert with vimg")
            .map_err(|e| anyhow!("setting MUIVerb: {e}"))?;
        key.set_string("ExtendedSubCommandsKey", "vimg.Menu")
            .map_err(|e| anyhow!("setting ExtendedSubCommandsKey: {e}"))?;
        Ok(())
    }

    fn write_command(path: &str, command: &str) -> Result<()> {
        let key = CURRENT_USER
            .create(path)
            .map_err(|e| anyhow!("creating {path}: {e}"))?;
        key.set_string("", command)
            .map_err(|e| anyhow!("setting default value: {e}"))?;
        Ok(())
    }
}
