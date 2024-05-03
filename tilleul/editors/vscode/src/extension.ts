import {spawnSync} from "child_process";
import type {
    ExtensionContext,
    WorkspaceConfiguration
} from "vscode";
import {
    commands,
    workspace
} from "vscode";
import type {
    Executable,
    LanguageClientOptions,
    ServerOptions
} from "vscode-languageclient/node";
import {
    LanguageClient
} from "vscode-languageclient/node";

let client: LanguageClient | null = null;

const SUCESS_STATUS = 0;

function getServerPath (config: Readonly<WorkspaceConfiguration>): string {
    const givenPath = config.get<string>("serverPath");

    if (givenPath === undefined) {
        throw new Error("`proost-lsp.serverPath` is illed-formed");
    }

    const result = spawnSync(givenPath);
    if (result.status === SUCESS_STATUS) {
        return givenPath;
    }

    const statusMessage = result.status !== null ? [`return status: ${result.status}`] : [];
    const errorMessage = result.error?.message !== undefined ? [`error: ${result.error.message}`] : [];
    throw new Error(
        `\`proost-lsp.serverPath\` (${givenPath}) does not point to a valid proost-LSP binary
        Failed to launch "${givenPath}"\n\t${[statusMessage, errorMessage].flat().join("\n\t")}`
    );
}

async function startClient (context: Readonly<ExtensionContext>): Promise<void> {
    const config = workspace.getConfiguration("proost-lsp");
    const serverPath = getServerPath(config);
	
    const executable: Executable = {
        command: serverPath
    };

    const serverOptions: ServerOptions = {
        run: executable,
        debug: executable
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{scheme: "file", language: "madelaine"}],
        initializationOptions: config
    };

    client = new LanguageClient("proost-lsp", serverOptions, clientOptions);

    context.subscriptions.push(commands.registerCommand("proost-lsp.restartServer", () => {
        void client?.restart();
    }));

    return client.start();
}

export async function activate (context: Readonly<ExtensionContext>): Promise<void> {
    return startClient(context).catch((e) => {
        console.log(e);
    });
}

export async function deactivate (): Promise<void> {
    return client?.stop();
}