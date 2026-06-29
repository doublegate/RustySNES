import sys

code = """
        if val != 0 {
            for ch in 0..8 {
                if val & (1 << ch) != 0 {
                    let bb = self.channels[ch].b_bank;
                    let b = self.channels[ch].b_address(0);
                    if b == 0x18 {
                        let a_bank = self.channels[ch].source_bank;
                        let a_addr = self.channels[ch].source_addr;
                        let mode = self.channels[ch].mode();
                        let count = self.channels[ch].count_or_indirect;
                        println!("DMA to VRAM! Bank: {:02x}, Addr: {:04x}, Count: {:04x}, Mode: {}", a_bank, a_addr, count, mode);
                    }
                }
            }
        }
"""
print("Need to inject this into dma.rs write_reg for 0x420B...")
