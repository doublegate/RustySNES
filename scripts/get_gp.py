with open("crates/rustysnes-core/src/dma.rs") as f:
    in_gp = False
    for line in f:
        if "pub fn run_gp" in line:
            in_gp = True
        if in_gp:
            print(line.rstrip())
            if "fn hdma_run" in line:
                break
