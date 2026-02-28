/**
 * Widget Helpers - Shared utilities for TUI widgets
 * 
 * Used by:
 * - Planning extension (researcher progress)
 * - Implementation extension (task progress, checkpoints)
 */

export interface WidgetLine {
	text: string;
	style?: "normal" | "dim" | "accent" | "success" | "warning" | "error";
}

export type WidgetContent = WidgetLine[];

/**
 * Format a progress widget for researcher status
 */
export function formatResearcherWidget(
	researchers: Array<{
		name: string;
		status: "waiting" | "running" | "complete" | "error";
		turns?: number;
	}>
): WidgetContent {
	const lines: WidgetContent = [
		{ text: "📋 Planning Agent Progress", style: "accent" },
		{ text: "─".repeat(40), style: "dim" },
	];

	for (const researcher of researchers) {
		const icon = getStatusIcon(researcher.status);
		const style = getStatusStyle(researcher.status);
		const turns = researcher.turns ? ` (${researcher.turns} turns)` : "";
		const statusText = researcher.status === "running" ? " running..." : "";

		lines.push({
			text: `${icon} ${researcher.name}${turns}${statusText}`,
			style,
		});
	}

	return lines;
}

/**
 * Format a progress widget for implementation workflow
 */
export function formatImplementationWidget(
	featureName: string,
	phases: Array<{
		name: string;
		tasks: Array<{
			description: string;
			status: "pending" | "current" | "complete";
		}>;
	}>,
	currentTask?: string
): WidgetContent {
	const lines: WidgetContent = [
		{ text: `🔧 Implementation: ${featureName}`, style: "accent" },
		{ text: "─".repeat(40), style: "dim" },
	];

	let totalTasks = 0;
	let completedTasks = 0;

	for (const phase of phases) {
		lines.push({ text: `Phase: ${phase.name}`, style: "accent" });

		for (const task of phase.tasks) {
			totalTasks++;
			if (task.status === "complete") completedTasks++;

			const icon = task.status === "complete" ? "✓" : task.status === "current" ? "⏳" : "○";
			const style = task.status === "complete" ? "success" : task.status === "current" ? "accent" : "dim";

			lines.push({
				text: `${icon} ${task.description}${task.status === "current" ? " (current)" : ""}`,
				style,
			});
		}

		lines.push({ text: "", style: "normal" });
	}

	// Progress summary
	const percentage = totalTasks > 0 ? Math.round((completedTasks / totalTasks) * 100) : 0;
	lines.push({ text: `Progress: ${completedTasks}/${totalTasks} (${percentage}%)`, style: "dim" });

	return lines;
}

/**
 * Format a checkpoint widget
 */
export function formatCheckpointWidget(
	checkpointId: string,
	questions: Array<{
		researcher: string;
		question: string;
	}>
): WidgetContent {
	const lines: WidgetContent = [
		{ text: `⏸ Checkpoint ${checkpointId}`, style: "warning" },
		{ text: "─".repeat(40), style: "dim" },
	];

	for (const q of questions) {
		lines.push({ text: `[${q.researcher}] ${q.question}`, style: "normal" });
		lines.push({ text: "", style: "normal" });
	}

	lines.push({ text: "Press Enter to continue, or 'r' to re-consult", style: "dim" });

	return lines;
}

/**
 * Format a simple status line for footer
 */
export function formatStatusLine(
	type: "planning" | "implementation",
	current: number,
	total: number,
	message?: string
): string {
	const icon = type === "planning" ? "📋" : "🔧";
	const progress = `${current}/${total}`;
	const suffix = message ? ` - ${message}` : "";
	return `${icon} ${type === "planning" ? "Planning" : "Implementing"}: ${progress}${suffix}`;
}

// Helper functions

function getStatusIcon(status: "waiting" | "running" | "complete" | "error"): string {
	switch (status) {
		case "waiting": return "○";
		case "running": return "⏳";
		case "complete": return "✓";
		case "error": return "✗";
	}
}

function getStatusStyle(status: "waiting" | "running" | "complete" | "error"): WidgetLine["style"] {
	switch (status) {
		case "waiting": return "dim";
		case "running": return "accent";
		case "complete": return "success";
		case "error": return "error";
	}
}

/**
 * Convert WidgetContent to string array for ctx.ui.setWidget()
 */
export function widgetToStringArray(content: WidgetContent): string[] {
	return content.map(line => {
		// Note: Actual styling is handled by the theme in the extension
		return line.text;
	});
}

/**
 * Theme-aware widget renderer
 * Extensions should pass their theme functions
 */
export function renderWidget(
	content: WidgetContent,
	theme: {
		fg: (color: string, text: string) => string;
	}
): string[] {
	return content.map(line => {
		if (line.style && line.style !== "normal") {
			const colorMap: Record<Exclude<WidgetLine["style"], "normal">, string> = {
				dim: "dim",
				accent: "accent",
				success: "success",
				warning: "warning",
				error: "error",
			};
			return theme.fg(colorMap[line.style as keyof typeof colorMap], line.text);
		}
		return line.text;
	});
}
