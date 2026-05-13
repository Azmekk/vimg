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
        println!("vimg context menu removed.");
        Ok(())
    }

    fn write_shared_menus(exe: &str) -> Result<()> {
        write_verb(
            &format!(r"{SHARED_MENU_KEY}\shell\01-png"),
            "Convert to PNG",
            &format!("\"{exe}\" \"%1\" -f png"),
            None,
        )?;
        write_verb(
            &format!(r"{SHARED_MENU_KEY}\shell\02-jpg"),
            "Convert to JPG",
            &format!("\"{exe}\" \"%1\" -f jpg"),
            None,
        )?;
        write_verb(
            &format!(r"{SHARED_MENU_KEY}\shell\03-webp"),
            "Convert to WebP",
            &format!("\"{exe}\" \"%1\" -f webp"),
            None,
        )?;
        write_verb(
            &format!(r"{SHARED_MENU_KEY}\shell\04-avif"),
            "Convert to AVIF",
            &format!("\"{exe}\" \"%1\" -f avif"),
            None,
        )?;
        // "Optimize" — preceded by a separator (CommandFlags = 0x40).
        write_verb(
            &format!(r"{SHARED_MENU_KEY}\shell\05-opt"),
            "Optimize",
            &format!("\"{exe}\" \"%1\""),
            Some(0x40),
        )?;
        Ok(())
    }

    fn write_verb(
        verb_path: &str,
        label: &str,
        command: &str,
        command_flags: Option<u32>,
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
        let exe = exe_path()?;
        key.set_string("Icon", &format!("{exe},0"))
            .map_err(|e| anyhow!("setting Icon: {e}"))?;
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
