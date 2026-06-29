with open("crates/rustysnes-core/src/dma.rs", "r") as f:
    for i, line in enumerate(f):
        if "fn step" in line:
            print(f"{i}: {line.strip()}")
