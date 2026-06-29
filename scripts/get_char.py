with open("ref-proj/ares/ares/sfc/coprocessor/superfx/core.cpp", "r") as f:
    for line in f:
        if "charAddress" in line:
            print(line.strip())
