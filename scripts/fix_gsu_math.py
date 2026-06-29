import re

with open('crates/rustysnes-cart/src/coproc/gsu.rs', 'r') as f:
    content = f.read()

# Replace i32::from(self.r[n]) with (self.r[n] as i16 as i32)
# But only in the i_add, i_sub, i_mult, etc functions where needed.
# Actually, I will just use sed or multi_replace_file_content!
