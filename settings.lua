return {
    lspsettings = {
        rust = {
            ["rust-analyzer"] = {
                cargo = {
                    features = {
                        "notify",
                        "gui"
                    }
                }
            }
        }
    }
}
