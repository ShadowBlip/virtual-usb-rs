use std::{error::Error, process::Command};

/// Loads the vhci-hcd module
pub fn load_vhci_hcd() -> Result<(), Box<dyn Error>> {
    let status = Command::new("modprobe").arg("vhci-hcd").status()?;
    if status.success() {
        return Ok(());
    }

    Err("Failed to load vhci-hcd module".into())
}
