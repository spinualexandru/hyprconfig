import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import type { Keybind } from "@/types/keybinds";
import {
	AVAILABLE_DISPATCHERS,
	LUA_ONLY_DISPATCHERS,
	MODIFIER_OPTIONS,
} from "@/constants/keybinds";
import { useConfigInfo } from "@/hooks/use-config-info";
import { getDispatcherDescription } from "@/utils/keybinds";

interface KeybindFormDialogProps {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	editingKeybind?: { keybind: Keybind; index: number } | null;
	onSuccess: () => void;
	isBindu?: boolean;
}

export function KeybindFormDialog({
	open,
	onOpenChange,
	...formProps
}: KeybindFormDialogProps) {
	const configInfo = useConfigInfo();

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent className="max-w-2xl">
				{/* Content unmounts while closed, so the form starts fresh on every open */}
				<KeybindForm
					onOpenChange={onOpenChange}
					isLua={configInfo?.format === "lua"}
					{...formProps}
				/>
			</DialogContent>
		</Dialog>
	);
}

function KeybindForm({
	onOpenChange,
	editingKeybind,
	onSuccess,
	isBindu = false,
	isLua,
}: Omit<KeybindFormDialogProps, "open"> & { isLua: boolean }) {
	const initial = editingKeybind?.keybind;
	const [formModifiers, setFormModifiers] = useState<string[]>(
		initial?.modifiers ?? [],
	);
	const [formKey, setFormKey] = useState(initial?.key ?? "");
	const [formDispatcher, setFormDispatcher] = useState(
		initial?.dispatcher ?? "",
	);
	const [formParams, setFormParams] = useState(initial?.params ?? "");
	const [formError, setFormError] = useState<string | null>(null);
	const [formLoading, setFormLoading] = useState(false);
	const [dispatcherSuggestions, setDispatcherSuggestions] = useState<string[]>(
		[],
	);
	const isLuaDispatcher = formDispatcher.trim() === "lua";

	const toggleModifier = (modifier: string) => {
		if (formModifiers.includes(modifier)) {
			setFormModifiers(formModifiers.filter((m) => m !== modifier));
		} else {
			setFormModifiers([...formModifiers, modifier]);
		}
	};

	const handleDispatcherChange = (value: string) => {
		setFormDispatcher(value);
		// Update suggestions based on input
		if (value.trim()) {
			const dispatchers = isLua
				? [...AVAILABLE_DISPATCHERS, ...LUA_ONLY_DISPATCHERS]
				: AVAILABLE_DISPATCHERS;
			const filtered = dispatchers
				.filter((d) => d.toLowerCase().includes(value.toLowerCase()))
				.slice(0, 8);
			setDispatcherSuggestions(filtered);
		} else {
			setDispatcherSuggestions([]);
		}
	};

	const handleSubmit = async () => {
		// Validation
		if (!formKey.trim()) {
			setFormError("Key is required");
			return;
		}
		if (!formDispatcher.trim()) {
			setFormError("Dispatcher is required");
			return;
		}

		setFormLoading(true);
		setFormError(null);

		// Bindu only supports add (no edit command), regular keybinds support both
		const command = isBindu
			? "add_bindu"
			: editingKeybind
				? "edit_keybind"
				: "add_keybind";

		const args =
			!isBindu && editingKeybind
				? {
						index: editingKeybind.index,
						modifiers: formModifiers,
						key: formKey.trim(),
						dispatcher: formDispatcher.trim(),
						params: formParams.trim(),
					}
				: {
						modifiers: formModifiers,
						key: formKey.trim(),
						dispatcher: formDispatcher.trim(),
						params: formParams.trim(),
					};

		invoke(command, args)
			.then(() => {
				onOpenChange(false);
				onSuccess();
			})
			.catch((err) => {
				setFormError(err as string);
			})
			.finally(() => {
				setFormLoading(false);
			});
	};

	return (
		<>
			<DialogHeader>
				<DialogTitle>
					{isBindu
						? "Add Universal Bind"
						: editingKeybind
							? "Edit Keybind"
							: "Add New Keybind"}
				</DialogTitle>
				<DialogDescription>
					{isBindu
						? "Create a keybind that works across all submaps."
						: editingKeybind
							? "Modify the keybind settings below."
							: "Create a new keyboard shortcut for Hyprland."}
				</DialogDescription>
			</DialogHeader>
			<div className="space-y-4 py-4">
				{/* Modifiers */}
				<div className="space-y-2">
					<Label>Modifiers (optional)</Label>
					<div className="flex gap-4">
						{MODIFIER_OPTIONS.map((mod) => (
							<div key={mod} className="flex items-center space-x-2">
								<Checkbox
									id={`mod-${mod}`}
									checked={formModifiers.includes(mod)}
									onCheckedChange={() => toggleModifier(mod)}
									disabled={formLoading}
								/>
								<Label
									htmlFor={`mod-${mod}`}
									className="text-sm font-normal cursor-pointer"
								>
									{mod}
								</Label>
							</div>
						))}
					</div>
				</div>

				{/* Key */}
				<div className="space-y-2">
					<Label htmlFor="key">Key *</Label>
					<Input
						id="key"
						placeholder="e.g., Q, Return, 1, left, F1"
						value={formKey}
						onChange={(e) => {
							setFormKey(e.target.value);
							setFormError(null);
						}}
						disabled={formLoading}
					/>
					<p className="text-xs text-muted-foreground">
						Examples: Q, Return, Escape, left, right, up, down, F1-F12, 1-9
					</p>
				</div>

				{/* Dispatcher */}
				<div className="space-y-2">
					<Label htmlFor="dispatcher">Dispatcher (Action) *</Label>
					<Input
						id="dispatcher"
						placeholder="e.g., exec, killactive, workspace"
						value={formDispatcher}
						onChange={(e) => handleDispatcherChange(e.target.value)}
						disabled={formLoading}
						list="dispatcher-suggestions"
					/>
					<datalist id="dispatcher-suggestions">
						{dispatcherSuggestions.map((d) => (
							<option key={d} value={d}>
								{getDispatcherDescription(d)}
							</option>
						))}
					</datalist>
					<p className="text-xs text-muted-foreground">
						Common: exec, killactive, workspace, togglefloating, fullscreen
						{isLua && (
							<>
								. Use <code>lua</code> for any other hl.dsp.* call
							</>
						)}
					</p>
				</div>

				{/* Parameters */}
				<div className="space-y-2">
					<Label htmlFor="params">
						{isLuaDispatcher ? "Lua expression *" : "Parameters (optional)"}
					</Label>
					<Input
						id="params"
						placeholder={
							isLuaDispatcher
								? 'e.g., hl.dsp.window.fullscreen({ mode = "maximized" })'
								: "e.g., kitty, 1, l"
						}
						value={formParams}
						onChange={(e) => setFormParams(e.target.value)}
						disabled={formLoading}
					/>
					<p className="text-xs text-muted-foreground">
						{isLuaDispatcher
							? "Written to the config as-is: an hl.dsp.* call or a function"
							: isLua
								? "Command arguments. Can use variables like $terminal or ${terminal}"
								: "Command arguments. Can use variables like $terminal"}
					</p>
				</div>

				{formError && <p className="text-sm text-destructive">{formError}</p>}
			</div>
			<DialogFooter>
				<Button
					variant="outline"
					onClick={() => onOpenChange(false)}
					disabled={formLoading}
				>
					Cancel
				</Button>
				<Button onClick={handleSubmit} disabled={formLoading}>
					{formLoading
						? "Adding..."
						: isBindu
							? "Add Universal Bind"
							: editingKeybind
								? "Save Changes"
								: "Add Keybind"}
				</Button>
			</DialogFooter>
		</>
	);
}
