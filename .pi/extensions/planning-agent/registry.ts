/**
 * Researcher Registry - Dispatch logic for planning extension
 */

import {
	discoverResearchers,
	matchResearchers,
	type ResearcherConfig,
	type DispatchOptions,
	type ResearcherOutput,
	dispatchResearchers,
} from "../lib/researcher-dispatch.js";

export { discoverResearchers, matchResearchers, dispatchResearchers };
export type { ResearcherConfig, DispatchOptions, ResearcherOutput };

/**
 * Analyze query and determine which researchers to dispatch
 */
export function analyzeQuery(query: string, researchers: ResearcherConfig[]): {
	matched: string[];
	signals: string[];
	always: string[];
} {
	const queryLower = query.toLowerCase();
	const matched: string[] = [];
	const signals: string[] = [];
	const always: string[] = [];

	for (const researcher of researchers) {
		if (researcher.always) {
			always.push(researcher.name);
			continue;
		}

		for (const signal of researcher.triggerSignals) {
			if (queryLower.includes(signal.toLowerCase())) {
				matched.push(researcher.name);
				signals.push(signal);
				break;
			}
		}
	}

	return { matched, signals, always: [...new Set(always)] };
}

/**
 * Build context string for researcher dispatch
 */
export function buildResearcherContext(
	projectInfo: {
		name?: string;
		tech?: string[];
		existingPatterns?: string[];
	},
	existingCode?: string
): string {
	const parts: string[] = [];

	if (projectInfo.name) {
		parts.push(`Project: ${projectInfo.name}`);
	}

	if (projectInfo.tech && projectInfo.tech.length > 0) {
		parts.push(`Tech Stack: ${projectInfo.tech.join(", ")}`);
	}

	if (projectInfo.existingPatterns && projectInfo.existingPatterns.length > 0) {
		parts.push(`Existing Patterns:\n${projectInfo.existingPatterns.map(p => `- ${p}`).join("\n")}`);
	}

	if (existingCode) {
		parts.push(`Relevant Code:\n\`\`\`\n${existingCode}\n\`\`\``);
	}

	return parts.join("\n\n");
}

/**
 * Get researcher display order for widgets
 */
export function getResearcherDisplayOrder(): string[] {
	return [
		"security",
		"ux-ui",
		"performance",
		"data-database",
		"api",
		"accessibility",
		"testing",
		"architecture",
		"devops",
		"compliance",
		"workflow-synthesizer",
	];
}

/**
 * Sort researchers for display
 */
export function sortResearchersForDisplay(researchers: string[]): string[] {
	const order = getResearcherDisplayOrder();
	return researchers.sort((a, b) => {
		const aIndex = order.indexOf(a);
		const bIndex = order.indexOf(b);
		if (aIndex === -1 && bIndex === -1) return a.localeCompare(b);
		if (aIndex === -1) return 1;
		if (bIndex === -1) return -1;
		return aIndex - bIndex;
	});
}
