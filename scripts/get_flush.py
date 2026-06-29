with open("ref-proj/ares/ares/sfc/coprocessor/superfx/core.cpp", "r") as f:
    in_flush = False
    for line in f:
        if "auto SuperFX::flushPixelCache" in line:
            in_flush = True
        if in_flush:
            print(line.rstrip())
            if line.startswith("}"):
                break
