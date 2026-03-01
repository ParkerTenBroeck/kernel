use std::{env, ffi::OsString, path::PathBuf, process::{Command, ExitCode}};
use std::{
    fs,
    io,
    path::{Path},
};

use elf::endian::AnyEndian;
use elf::ElfBytes;

const TARGET: &str = "riscv64gc-unknown-none-elf";

fn rustflags_string() -> String {
    let flags: &[&str] = &[
        "-C", "link-arg=--emit-relocs",
        "-C", "link-arg=-Tlink.map",
        // "-C", "link-arg=-r",
        "-C", "link-arg=-pie",
        "-C", "code-model=medium",
        "-C", "relocation-model=pic",
        "-C", "target-feature=+zihintpause",
        "-C", "force-frame-pointers=true",
    ];
    flags.join(" ")
}

fn kernel_elf_path(bin: &str, profile: &str) -> PathBuf {
    PathBuf::from("target")
        .join(TARGET)
        .join(profile)
        .join(bin)
}

fn format_command_for_print(cmd: &Command) -> Option<String> {
    let program = cmd.get_program().to_string_lossy().to_string();
    let args = cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    Some(std::iter::once(program).chain(args).collect::<Vec<_>>().join(" "))
}

fn run_cmd(cmd: &mut Command) -> Result<(), String> {
    eprintln!(
        "+ {}",
        format_command_for_print(cmd)
            .unwrap_or_else(|| "<command>".to_string())
    );

    let status = cmd.status().map_err(|e| format!("failed to spawn: {e}"))?;
    if !status.success() {
        return Err(format!("command exited with status: {status}"));
    }
    Ok(())
}

fn cargo_nightly() -> OsString {
    if let Ok(v) = env::var("CARGO_NIGHTLY") {
        return OsString::from(v);
    }
    OsString::from("cargo")
}

fn build(bin: &str, release: bool) -> Result<(), String> {
    let mut cmd = Command::new(cargo_nightly());

    if cmd.get_program() == "cargo" {
        cmd.arg("+nightly");
    }

    cmd.arg("build")
        .arg("-Z")
        .arg("build-std=core,compiler_builtins,alloc")
        .arg("-Z")
        .arg("build-std-features=compiler-builtins-mem")
        .arg("--target")
        .arg(TARGET)
        .arg("--bin")
        .arg(bin);

    if release {
        cmd.arg("--release");
    }

    cmd.env("RUSTFLAGS", rustflags_string());

    run_cmd(&mut cmd)
}


fn main() -> ExitCode {
    let mut release = false;
    let mut passthrough = false;
    for a in env::args().skip(1) {
        if !passthrough && a == "--release" {
            release = true;
        } else if !passthrough && a == "--" {
            passthrough = true;
        }
    }

    let result = build("kernel", release);

    let path = kernel_elf_path("kernel", if release {"release"} else {"debug"});
    // relink(path);
    
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

fn relink(path: PathBuf){
    emit_and_relink(
    path,
    "link.map".into(),
    "riscv64-none-elf-objcopy",
    "ld.lld",
    "elf64-littleriscv",
    "riscv",
    ).expect("msg");
}


#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct KSym {
    addr: u64,
    name_off: u32,
    size: u32,
}

fn build_ksyms_from_elf(elf_path: &Path) -> io::Result<(Vec<u8>, Vec<u8>)> {
    let file_data = fs::read(elf_path)?;
    let file = ElfBytes::<AnyEndian>::minimal_parse(&file_data)
        .map_err(|e| io::Error::other(format!("ELF parse: {e}")))?;

    let common = file
        .find_common_data()
        .map_err(|e| io::Error::other(format!("common data: {e}")))?;

    let syms = common
        .symtab
        .ok_or_else(|| io::Error::other("no .symtab found"))?;
    let strtab = common
        .symtab_strs
        .ok_or_else(|| io::Error::other("no .strtab for .symtab found"))?;

    let mut kstrtab: Vec<u8> = vec![0];
    let mut ksym_entries: Vec<KSym> = Vec::new();

    for sym in syms {
        if sym.st_symtype() != 0x2 {
            continue;
        }
        
        if sym.st_value == 0 {
            continue;
        }

        let raw_name = strtab.get(sym.st_name as usize).unwrap_or("<UNDEFINED>");
        let demangled = format!("{:#}", rustc_demangle::demangle(raw_name));

        let name_off = kstrtab.len();
        kstrtab.extend_from_slice(demangled.as_bytes());
        kstrtab.push(0);

        let addr = sym.st_value;
        let size = sym.st_size as u32;

        if name_off > u32::MAX as usize {
            return Err(io::Error::other(
                "kstrtab too large for u32 offsets",
            ));
        }

        ksym_entries.push(KSym {
            addr,
            name_off: name_off as u32,
            size,
        });
    }

    let mut ksymtab: Vec<u8> = Vec::with_capacity(ksym_entries.len() * 16);
    for e in &ksym_entries {
        ksymtab.extend_from_slice(&e.addr.to_le_bytes());
        ksymtab.extend_from_slice(&e.name_off.to_le_bytes());
        ksymtab.extend_from_slice(&e.size.to_le_bytes());
    }

    Ok((ksymtab, kstrtab))
}

pub fn emit_and_relink(
    existing_elf: PathBuf,
    linker_script: PathBuf,
    objcopy: &str,
    ld: &str,
    objcopy_bfdname: &str,
    objcopy_arch: &str,
) -> Result<(), String> {
    let tmp_base = std::env::temp_dir().join(format!(
        "ksyms-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&tmp_base).unwrap();

    let (ksymtab_bin, kstrtab_bin) = build_ksyms_from_elf(&existing_elf).unwrap();

    let ksymtab_path = tmp_base.join("ksymtab.bin");
    let kstrtab_path = tmp_base.join("kstrtab.bin");
    fs::write(&ksymtab_path, &ksymtab_bin).unwrap();
    fs::write(&kstrtab_path, &kstrtab_bin).unwrap();

    let ksymtab_o = tmp_base.join("ksymtab.o");
    let kstrtab_o = tmp_base.join("kstrtab.o");

    run_cmd(
        Command::new(objcopy)
            .arg("-I")
            .arg("binary")
            .arg("-O")
            .arg(objcopy_bfdname)
            .arg("-B")
            .arg(objcopy_arch)
            .arg("--rename-section")
            .arg(".data=.ksymtab,alloc,load,readonly,data")
            .arg(&ksymtab_path)
            .arg(&ksymtab_o),
    )?;

    run_cmd(
        Command::new(objcopy)
            .arg("-I")
            .arg("binary")
            .arg("-O")
            .arg(objcopy_bfdname)
            .arg("-B")
            .arg(objcopy_arch)
            .arg("--rename-section")
            .arg(".data=.kstrtab,alloc,load,readonly,data")
            .arg(&kstrtab_path)
            .arg(&kstrtab_o),
    )?;

    let mut cmd = Command::new(ld);
    cmd.arg("-T").arg(&linker_script);
    cmd.arg("-pie");

    let mut output = existing_elf.clone();
    output.add_extension("elf");
    cmd.arg("-o").arg(&output);

    cmd.arg(&existing_elf).arg(&ksymtab_o).arg(&kstrtab_o);

    run_cmd(&mut cmd)?;

    eprintln!("Artifacts kept at: {}", tmp_base.display());
    Ok(())
}