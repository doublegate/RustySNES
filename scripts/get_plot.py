with open("ref-proj/ares/ares/sfc/coprocessor/superfx/core.cpp", "r") as f:
    for i, line in enumerate(f):
        if "auto SuperFX::plot" in line:
            for j in range(30):
                print(f.readline().rstrip())
            break
