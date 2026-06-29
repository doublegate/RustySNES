with open("crates/rustysnes-core/src/dma.rs") as f:
    in_hdma = False
    for line in f:
        if "pub fn hdma_run" in line:
            in_hdma = True
        if in_hdma:
            print(line.rstrip())
            if "fn hdma_update" in line:
                break
