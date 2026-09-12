/// Copies text, falling back to a scratch textarea: the Clipboard API needs
/// a secure context and can be unavailable in the webview, and a silent
/// no-op would look like the copy button is broken.
export async function copyText(text: string) {
	try {
		await navigator.clipboard.writeText(text);
		return;
	} catch {
		const scratch = document.createElement("textarea");
		scratch.value = text;
		document.body.appendChild(scratch);
		scratch.select();
		document.execCommand("copy");
		scratch.remove();
	}
}
