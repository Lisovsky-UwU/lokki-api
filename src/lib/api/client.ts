import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import type {
	CollectionSummary,
	CollectionTreeNode,
	EnvironmentEntry,
	EnvironmentFile,
	EnvironmentScope,
	ExecutionOutcome,
	HttpMethod,
	Id,
	RequestFile,
	WorkspaceFile,
} from "../bindings/types";

export interface OpenWorkspaceResult {
	workspace: WorkspaceFile;
	collections: CollectionSummary[];
}

export const api = {
	pickWorkspaceFolder: (): Promise<string | null> =>
		openDialog({ directory: true, multiple: false, title: "Open workspace folder" }) as Promise<string | null>,

	openWorkspace: (path: string) => invoke<OpenWorkspaceResult>("open_workspace", { path }),
	getLastWorkspace: () => invoke<string | null>("get_last_workspace"),
	listCollections: (workspacePath: string) =>
		invoke<CollectionSummary[]>("list_collections", { workspacePath }),
	createCollection: (workspacePath: string, name: string) =>
		invoke<CollectionSummary>("create_collection", { workspacePath, name }),
	loadCollectionTree: (collectionPath: string) =>
		invoke<CollectionTreeNode>("load_collection_tree", { collectionPath }),

	loadRequest: (requestPath: string) => invoke<RequestFile>("load_request", { requestPath }),
	saveRequest: (requestPath: string, request: RequestFile) =>
		invoke<RequestFile>("save_request", { requestPath, request }),
	createRequest: (parentPath: string, name: string, method: HttpMethod) =>
		invoke<RequestFile>("create_request", { parentPath, name, method }),
	deleteRequest: (requestPath: string) => invoke<void>("delete_request", { requestPath }),
	createFolder: (parentPath: string, name: string) =>
		invoke<string>("create_folder", { parentPath, name }),
	deleteFolder: (folderPath: string) => invoke<void>("delete_folder", { folderPath }),

	listEnvironments: (rootPath: string) => invoke<EnvironmentEntry[]>("list_environments", { rootPath }),
	createEnvironment: (rootPath: string, name: string, scope: EnvironmentScope) =>
		invoke<EnvironmentEntry>("create_environment", { rootPath, name, scope }),
	saveEnvironment: (envPath: string, environment: EnvironmentFile) =>
		invoke<EnvironmentFile>("save_environment", { envPath, environment }),
	setActiveEnvironment: (rootPath: string, environmentId: Id | null) =>
		invoke<void>("set_active_environment", { rootPath, environmentId }),
	getActiveEnvironment: (rootPath: string) =>
		invoke<EnvironmentEntry | null>("get_active_environment", { rootPath }),

	sendRequest: (request: RequestFile, collectionPath: string) =>
		invoke<ExecutionOutcome>("send_request", { request, collectionPath }),

	setSecret: (workspacePath: string, variableId: Id, value: string) =>
		invoke<void>("set_secret", { workspacePath, variableId, value }),
	revealSecret: (workspacePath: string, variableId: Id) =>
		invoke<string | null>("reveal_secret", { workspacePath, variableId }),
};
