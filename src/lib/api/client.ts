import { invoke } from "@tauri-apps/api/core";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import type {
	AppInfo,
	CollectionSummary,
	CollectionTreeNode,
	EnvironmentEntry,
	EnvironmentFile,
	EnvironmentScope,
	ExecutionOutcome,
	HttpMethod,
	Id,
	RequestAtPath,
	RequestFile,
	RequestSettings,
	WorkspaceFile,
} from "../bindings/types";

export interface OpenWorkspaceResult {
	workspace: WorkspaceFile;
	collections: CollectionSummary[];
}

export const api = {
	appInfo: () => invoke<AppInfo>("app_info"),
	getRequestSettings: () => invoke<RequestSettings>("get_request_settings"),
	saveRequestSettings: (settings: RequestSettings) =>
		invoke<RequestSettings>("save_request_settings", { settings }),

	pickWorkspaceFolder: (): Promise<string | null> =>
		openDialog({ directory: true, multiple: false, title: "Open workspace folder" }) as Promise<string | null>,

	openWorkspace: (path: string) => invoke<OpenWorkspaceResult>("open_workspace", { path }),
	createWorkspace: (path: string, name: string) =>
		invoke<OpenWorkspaceResult>("create_workspace", { path, name }),
	renameWorkspace: (path: string, newName: string) =>
		invoke<WorkspaceFile>("rename_workspace", { path, newName }),
	getLastWorkspace: () => invoke<string | null>("get_last_workspace"),
	listCollections: (workspacePath: string) =>
		invoke<CollectionSummary[]>("list_collections", { workspacePath }),
	createCollection: (workspacePath: string, name: string) =>
		invoke<CollectionSummary>("create_collection", { workspacePath, name }),
	loadCollectionTree: (collectionPath: string) =>
		invoke<CollectionTreeNode>("load_collection_tree", { collectionPath }),
	renameCollection: (collectionPath: string, newName: string) =>
		invoke<CollectionSummary>("rename_collection", { collectionPath, newName }),
	deleteCollection: (collectionPath: string) => invoke<void>("delete_collection", { collectionPath }),

	loadRequest: (requestPath: string) => invoke<RequestFile>("load_request", { requestPath }),
	saveRequest: (requestPath: string, request: RequestFile) =>
		invoke<RequestFile>("save_request", { requestPath, request }),
	createRequest: (parentPath: string, name: string, method: HttpMethod) =>
		invoke<RequestFile>("create_request", { parentPath, name, method }),
	cloneRequest: (requestPath: string, newName: string) =>
		invoke<RequestAtPath>("clone_request", { requestPath, newName }),
	deleteRequest: (requestPath: string) => invoke<void>("delete_request", { requestPath }),
	/// Files an in-memory request into a workspace folder, where it becomes
	/// an ordinary request with an identity of its own.
	adoptRequest: (parentPath: string, name: string, request: RequestFile) =>
		invoke<RequestAtPath>("adopt_request", { parentPath, name, request }),
	/// Writes an in-memory request to a file outside any workspace.
	exportRequest: (filePath: string, name: string, request: RequestFile) =>
		invoke<RequestFile>("export_request", { filePath, name, request }),
	/// Picks any file from disk to send as a request body.
	pickBodyFile: (): Promise<string | null> =>
		openDialog({ multiple: false, title: "Файл для тела запроса" }) as Promise<string | null>,
	/// Picks where to write a response body the user wants to keep.
	pickDownloadTarget: (defaultName: string): Promise<string | null> =>
		saveDialog({ title: "Сохранить ответ", defaultPath: defaultName }) as Promise<string | null>,
	saveResponseBody: (filePath: string, bodyBase64: string) =>
		invoke<void>("save_response_body", { filePath, bodyBase64 }),
	pickRequestFile: (defaultName: string): Promise<string | null> =>
		saveDialog({
			title: "Сохранить запрос",
			defaultPath: `${defaultName}.lokki.toml`,
			filters: [{ name: "Запрос LokkiAPI", extensions: ["toml"] }],
		}) as Promise<string | null>,
	renameRequest: (requestPath: string, newName: string) =>
		invoke<RequestAtPath>("rename_request", { requestPath, newName }),
	createFolder: (parentPath: string, name: string) =>
		invoke<string>("create_folder", { parentPath, name }),
	deleteFolder: (folderPath: string) => invoke<void>("delete_folder", { folderPath }),
	renameFolder: (folderPath: string, newName: string) =>
		invoke<string>("rename_folder", { folderPath, newName }),
	moveNode: (sourcePath: string, targetParent: string) =>
		invoke<string>("move_node", { sourcePath, targetParent }),
	reorderChildren: (orderedPaths: string[]) => invoke<void>("reorder_children", { orderedPaths }),

	listEnvironments: (rootPath: string) => invoke<EnvironmentEntry[]>("list_environments", { rootPath }),
	createEnvironment: (rootPath: string, name: string, scope: EnvironmentScope) =>
		invoke<EnvironmentEntry>("create_environment", { rootPath, name, scope }),
	// Returns the entry, not just the file: renaming an environment moves it.
	saveEnvironment: (envPath: string, environment: EnvironmentFile) =>
		invoke<EnvironmentEntry>("save_environment", { envPath, environment }),
	// Takes the workspace path too: deleting an environment also drops the
	// secret values its variables owned, and those live in the workspace.
	deleteEnvironment: (workspacePath: string, envPath: string) =>
		invoke<void>("delete_environment", { workspacePath, envPath }),
	setActiveEnvironment: (rootPath: string, environmentId: Id | null) =>
		invoke<void>("set_active_environment", { rootPath, environmentId }),
	getActiveEnvironment: (rootPath: string) =>
		invoke<EnvironmentEntry | null>("get_active_environment", { rootPath }),

	// `sendId` is minted per send by the caller so it can be cancelled. Both
	// paths are optional: an incognito request belongs to no collection, and
	// one started from the welcome screen has no workspace either — it then
	// resolves no variables at all.
	sendRequest: (
		request: RequestFile,
		workspacePath: string | null,
		collectionPath: string | null,
		sendId: string,
	) => invoke<ExecutionOutcome>("send_request", { request, workspacePath, collectionPath, sendId }),
	// Resolves to false when the send had already finished — a click and a
	// response can always cross paths, and that isn't an error.
	cancelSend: (sendId: string) => invoke<boolean>("cancel_send", { sendId }),

	setSecret: (workspacePath: string, variableId: Id, value: string) =>
		invoke<void>("set_secret", { workspacePath, variableId, value }),
	revealSecret: (workspacePath: string, variableId: Id) =>
		invoke<string | null>("reveal_secret", { workspacePath, variableId }),
};
