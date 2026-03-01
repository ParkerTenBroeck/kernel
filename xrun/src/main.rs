use std::{
    env,
    path::PathBuf,
    process::{Command, ExitCode},
};

const TARGET: &str = "riscv64gc-unknown-none-elf";


fn kernel_elf_path(bin: &str, profile: &str) -> PathBuf {
    PathBuf::from("target")
        .join(TARGET)
        .join(profile)
        .join(bin)
}

fn run_cmd(mut cmd: Command) -> Result<(), String> {
    eprintln!(
        "+ {}",
        format_command_for_print(&cmd)
            .unwrap_or_else(|| "<command>".to_string())
    );

    let status = cmd.status().map_err(|e| format!("failed to spawn: {e}"))?;
    if !status.success() {
        return Err(format!("command exited with status: {status}"));
    }
    Ok(())
}

fn format_command_for_print(cmd: &Command) -> Option<String> {
    let program = cmd.get_program().to_string_lossy().to_string();
    let args = cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    Some(std::iter::once(program).chain(args).collect::<Vec<_>>().join(" "))
}



fn qemu_args() -> Vec<String> {
    vec![
        "qemu-system-riscv64".into(),
        "-machine".into(),
        "virt,acpi=off".into(),
        "-smp".into(),
        "4".into(),
        "-m".into(),
        "512M".into(),
        "-chardev".into(),
        "stdio,id=uart0".into(),
        "-serial".into(),
        "chardev:uart0".into(),
        "-device".into(),
        "pci-testdev".into(),
        "-monitor".into(),
        "vc".into(),
        "-display".into(),
        "gtk".into(),
        "-device".into(),
        "VGA,id=vgadev".into(),
        "-drive".into(),
        "if=none,format=raw,readonly=off,file=run/fs.img,id=drv0".into(),
        "-device".into(),
        "virtio-blk-pci,drive=drv0".into(),
        "-netdev".into(),
        "user,id=net0".into(),
        "-device".into(),
        "i82559c,netdev=net0".into(),
        "-chardev".into(),
        "vc,id=pci_uart".into(),
        "-device".into(),
        "pci-serial,chardev=pci_uart".into(),
    ]
}

fn run(bin: &str, release: bool, extra_qemu_args: &[String]) -> Result<(), String> {
    let profile = if release { "release" } else { "debug" };
    let elf = kernel_elf_path(bin, profile);

    if !elf.is_file() {
        return Err(format!(
            "kernel ELF not found at {}. Build first.",
            elf.display()
        ));
    }

    let mut args = qemu_args();

    let program = args.remove(0);

    args.push("-kernel".into());
    args.push(elf.to_string_lossy().to_string());

    args.extend_from_slice(extra_qemu_args);

    let mut cmd = Command::new(program);
    cmd.args(args);

    run_cmd(cmd)
}


fn main() -> ExitCode {
    let mut release = false;
    let mut extra_qemu_args: Vec<String> = Vec::new();

    let mut build_command = Command::new("cargo");
    build_command.arg("xbuild");

    let mut passthrough = false;
    for a in env::args().skip(1) {
        if !passthrough && a == "--release" {
            release = true;
            build_command.arg("--").arg("--release");
        } else if !passthrough && a == "--" {
            passthrough = true;
        } else {
            extra_qemu_args.push(a);
        }
    }

    match run_cmd(build_command) {
        Ok(()) => {},
        Err(e) => {
            eprintln!("build error: {e}");
            return ExitCode::from(1);
        }
    }

    let result = run("kernel", release, &extra_qemu_args);

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}