use anyhow::Result;
use std::process::Command;

/// Demonstrates direct integration with BitVMX-CPU for option settlement
fn main() -> Result<()> {
    println!("BitVMX-CPU Direct Integration Example");
    println!("=====================================\n");

    // Option parameters
    // Format: [option_type(4), strike_price(4), spot_price(4), quantity(4)]
    // Call option, Strike: $50k, Spot: $52k, Quantity: 1.0
    let input_hex = "00000000404b4c0080584f0064000000";
    
    println!("📊 Option Parameters:");
    println!("  Type: Call");
    println!("  Strike: $50,000");
    println!("  Spot: $52,000");
    println!("  Quantity: 1.0");
    println!("  Input hex: {}", input_hex);
    
    // Path to the option settlement ELF
    let elf_path = "hello-world.elf";
    
    println!("\n🚀 Executing RISC-V program using emulator CLI...");
    
    // Execute the program using the emulator CLI (without input first)
    let output = Command::new("cargo")
        .current_dir("/Users/parkgeonwoo/oracle_vm/bitvmx_protocol/BitVMX-CPU")
        .args(&[
            "run",
            "--release",
            "-p",
            "emulator",
            "--",
            "execute",
            "--elf",
            elf_path,
            "--stdout"
        ])
        .output()?;
    
    if output.status.success() {
        println!("✅ Execution completed successfully!");
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("\n📤 Program Output:");
        println!("{}", stdout);
        
        // Parse trace information if available
        if stdout.contains("trace_size") {
            println!("\n🔍 Execution Trace Generated");
            println!("  This trace can be used for Bitcoin Script proof generation");
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("❌ Execution failed:");
        println!("{}", stderr);
    }
    
    println!("\n💡 Next Steps:");
    println!("  1. Generate ROM commitment using: cargo run -p emulator -- generate-rom-commitment --elf <file>");
    println!("  2. Create instruction mapping using: cargo run -p emulator -- instruction-mapping");
    println!("  3. Use bitcoin-script-riscv to convert trace to Bitcoin Script");
    println!("  4. Create pre-signed transaction for option settlement");
    
    // Demonstrate ROM commitment generation
    println!("\n📝 Generating ROM commitment...");
    let rom_output = Command::new("cargo")
        .current_dir("/Users/parkgeonwoo/oracle_vm/bitvmx_protocol/BitVMX-CPU")
        .args(&[
            "run",
            "--release",
            "-p",
            "emulator",
            "--",
            "generate-rom-commitment",
            "--elf",
            elf_path
        ])
        .output()?;
    
    if rom_output.status.success() {
        let rom_stdout = String::from_utf8_lossy(&rom_output.stdout);
        println!("✅ ROM Commitment generated:");
        // Extract commitment from output
        for line in rom_stdout.lines() {
            if line.contains("commitment") || line.contains("hash") {
                println!("  {}", line);
            }
        }
    }
    
    Ok(())
}