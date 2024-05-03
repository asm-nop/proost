# Madelaine extension for Visual Studio Code

This is an extension for Proost, including a syntactic coloration along with an LSP client for `tilleul`.

## Build

To build the extension, you have to have [`npm`](https://www.npmjs.com/), [`typescript`](https://www.typescriptlang.org/) and [`vsce`](https://github.com/microsoft/vscode-vsce) installed. You can compile the project by being at the root of this extension (the folder of this README), and run the following commands:

```bash
$ npm install
$ npm run compile
$ vsce package
```

This will create a `.vsix` package.

## Installation

Once you have the `.vsix` package, you can launch VSCode then go in `Extensions -> Views and More Actions -> Install from VSIX`. This will install the extension.

## Usage

This extension uses [`tilleul`](https://gitlab.crans.org/loutr/proost/-/tree/main/tilleul) as default backend, but any proost LSP can work.

You have to have either the `tilleul` binary in your `PATH` or to change the setting `proost-lsp.serverPath` for the absolute path to any proost LSP server binary.

You can always restart the LSP server by running the `Restart Proost LSP Server` command.
