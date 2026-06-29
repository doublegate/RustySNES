with open("crates/rustysnes-cart/src/coproc/gsu.rs", "r") as f:
    in_plot = False
    for line in f:
        if "fn plot" in line:
            in_plot = True
        if in_plot:
            print(line.rstrip())
            if line.startswith("    }"):
                break
