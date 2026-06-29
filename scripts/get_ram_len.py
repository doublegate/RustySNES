with open("crates/rustysnes-cart/src/coproc/superfx.rs") as f:
    for line in f:
        if "let ram_len =" in line:
            print(line.strip())
