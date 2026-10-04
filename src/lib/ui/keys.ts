/// The modifier the shortcut handlers accept as Ctrl, named the way this
/// keyboard labels it. The handlers take either key; only the hints differ.
export const MOD_KEY = /Mac|iPhone|iPad/.test(navigator.platform) ? "⌘" : "Ctrl";
