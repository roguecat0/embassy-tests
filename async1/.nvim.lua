local nvlsp = require("nvchad.configs.lspconfig")
vim.print("esp-rust-analyzer-config")
require("lspconfig").rust_analyzer.setup({
	on_attach = nvlsp.on_attach,
	on_init = nvlsp.on_init,
	capabilities = nvlsp.capabilities,
	settings = {
		["rust-analyzer"] = {
			cargo = {
				allTargets = false,
				target = "xtensa-esp32-none-elf",
				extraEnv = {
					RUSTUP_TOOLCHAIN = "esp",
				},
			},
			check = {
				extraEnv = {
					RUSTUP_TOOLCHAIN = "esp",
				},
			},
			server = {
				extraEnv = {
					RUSTUP_TOOLCHAIN = "stable",
				},
			},
		},
	},
})
