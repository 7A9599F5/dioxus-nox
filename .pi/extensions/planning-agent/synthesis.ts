/**
 * Synthesis - Merge researcher findings into SPEC.md
 */

import type { ResearcherOutput, ResearcherConcern } from "../lib/researcher-dispatch.js";
import { parseYaml, extractYamlBlock } from "../lib/yaml-parser.js";

export interface SynthesizedFindings {
	domainSummaries: Map<string, string>;
	allConcerns: SynthesizedConcern[];
	prioritizedQuestions: SynthesizedQuestion[];
	workflowHints: Map<string, WorkflowGuidance>;
	deferredDecisions: DeferredDecision[];
}

export interface SynthesizedConcern {
	id: string;
	researchers: string[];
	severity: "critical" | "high" | "medium" | "low";
	category: string;
	description: string;
	whyItMatters: string;
	defaultAssumption: string;
	questions: SynthesizedQuestion[];
}

export interface SynthesizedQuestion {
	id: string;
	question: string;
	researcher: string;
	options: QuestionOption[];
	hybridPossible: boolean;
	priority: number;
}

export interface QuestionOption {
	label: string;
	tradeoffs: string;
}

export interface WorkflowGuidance {
	phase: "foundation" | "core" | "integration" | "polish";
	order: number;
	tasks: string[];
	checkpointQuestions: string[];
	reconsultWhen: string[];
	testingMilestones: string[];
}

export interface DeferredDecision {
	id: string;
	question: string;
	context: string;
	options: string[];
	defaultIfUnresolved: string;
}

/**
 * Synthesize all researcher outputs into structured findings
 */
export function synthesizeFindings(researcherOutputs: Map<string, ResearcherOutput>): SynthesizedFindings {
	const domainSummaries = new Map<string, string>();
	const allConcerns: SynthesizedConcern[] = [];
	const workflowHints = new Map<string, WorkflowGuidance>();
	const questionMap = new Map<string, SynthesizedQuestion>();

	// Collect all data from researchers
	for (const [name, output] of researcherOutputs) {
		domainSummaries.set(name, output.domain_summary);

		// Extract workflow guidance
		if (output.workflow_guidance) {
			workflowHints.set(name, output.workflow_guidance);
		}

		// Process concerns
		for (const concern of output.concerns) {
			// Check for duplicate concerns (same category, similar description)
			const existingConcern = findSimilarConcern(allConcerns, concern);
			
			if (existingConcern) {
				// Merge into existing concern
				existingConcern.researchers.push(name);
				mergeQuestions(existingConcern.questions, concern.questions, name, questionMap);
			} else {
				// Create new synthesized concern
				const synthesized: SynthesizedConcern = {
					id: concern.id,
					researchers: [name],
					severity: concern.severity,
					category: concern.category,
					description: concern.description,
					whyItMatters: concern.why_it_matters,
					defaultAssumption: concern.default_assumption,
					questions: [],
				};

				mergeQuestions(synthesized.questions, concern.questions, name, questionMap);
				allConcerns.push(synthesized);
			}
		}
	}

	// Sort concerns by severity
	const severityOrder = { critical: 0, high: 1, medium: 2, low: 3 };
	allConcerns.sort((a, b) => severityOrder[a.severity] - severityOrder[b.severity]);

	// Prioritize questions
	const prioritizedQuestions = Array.from(questionMap.values()).sort((a, b) => {
		// Critical questions first, then by researcher priority
		const severityA = getQuestionSeverity(a.id, allConcerns);
		const severityB = getQuestionSeverity(b.id, allConcerns);
		return severityOrder[severityA] - severityOrder[severityB];
	});

	// Identify deferred decisions
	const deferredDecisions = identifyDeferredDecisions(allConcerns);

	return {
		domainSummaries,
		allConcerns,
		prioritizedQuestions,
		workflowHints,
		deferredDecisions,
	};
}

/**
 * Find a similar concern in the list
 */
function findSimilarConcern(concerns: SynthesizedConcern[], newConcern: ResearcherConcern): SynthesizedConcern | null {
	for (const concern of concerns) {
		if (concern.category === newConcern.category) {
			// Same category, check if description is similar
			const similarity = calculateSimilarity(concern.description, newConcern.description);
			if (similarity > 0.6) {
				return concern;
			}
		}
	}
	return null;
}

/**
 * Simple string similarity (Jaccard on words)
 */
function calculateSimilarity(a: string, b: string): number {
	const wordsA = new Set(a.toLowerCase().split(/\s+/));
	const wordsB = new Set(b.toLowerCase().split(/\s+/));
	const intersection = new Set([...wordsA].filter(x => wordsB.has(x)));
	const union = new Set([...wordsA, ...wordsB]);
	return intersection.size / union.size;
}

/**
 * Merge questions from a concern into the question list
 */
function mergeQuestions(
	target: SynthesizedQuestion[],
	questions: ResearcherConcern["questions"],
	researcher: string,
	questionMap: Map<string, SynthesizedQuestion>
): void {
	for (const q of questions) {
		const questionId = `${researcher}-${q.question.slice(0, 30).replace(/\s+/g, "-")}`;
		
		if (!questionMap.has(questionId)) {
			const synthesized: SynthesizedQuestion = {
				id: questionId,
				question: q.question,
				researcher,
				options: q.options,
				hybridPossible: q.hybrid_possible,
				priority: 0,
			};
			questionMap.set(questionId, synthesized);
			target.push(synthesized);
		}
	}
}

/**
 * Get severity for a question based on its concern
 */
function getQuestionSeverity(questionId: string, concerns: SynthesizedConcern[]): "critical" | "high" | "medium" | "low" {
	for (const concern of concerns) {
		if (concern.questions.some(q => q.id === questionId)) {
			return concern.severity;
		}
	}
	return "medium";
}

/**
 * Identify decisions that can be deferred to implementation
 */
function identifyDeferredDecisions(concerns: SynthesizedConcern[]): DeferredDecision[] {
	const deferred: DeferredDecision[] = [];

	for (const concern of concerns) {
		// Low severity concerns with default assumptions can often be deferred
		if (concern.severity === "low" && concern.defaultAssumption) {
			deferred.push({
				id: `DEFER-${concern.id}`,
				question: concern.questions[0]?.question || concern.description,
				context: concern.whyItMatters,
				options: concern.questions[0]?.options.map(o => o.label) || [concern.defaultAssumption],
				defaultIfUnresolved: concern.defaultAssumption,
			});
		}
	}

	return deferred;
}

/**
 * Generate SPEC.md content from synthesized findings
 */
export function generateSpecMd(
	featureName: string,
	findings: SynthesizedFindings,
	decisions: Map<string, string>
): string {
	const lines: string[] = [
		`# Design Specification: ${featureName}`,
		"",
		`**Created:** ${new Date().toISOString().split("T")[0]}`,
		`**Status:** [ ] Draft [ ] Approved [ ] Complete`,
		"",
		"## 1. Problem Statement",
		"",
		"[To be filled based on user input]",
		"",
		"## 2. Scope Definition",
		"",
		"### In Scope",
		"- [ ] [To be determined]",
		"",
		"### Out of Scope",
		"- [To be determined]",
		"",
		"## 3. Researcher Insights",
		"",
	];

	// Add researcher summaries
	for (const [researcher, summary] of findings.domainSummaries) {
		lines.push(`### ${formatResearcherName(researcher)}`);
		lines.push("");
		lines.push(summary);
		lines.push("");
	}

	// Add concerns
	lines.push("## 4. Key Concerns");
	lines.push("");

	for (const concern of findings.allConcerns.slice(0, 10)) { // Top 10
		lines.push(`### ${concern.id}: ${concern.category}`);
		lines.push("");
		lines.push(`**Severity:** ${concern.severity}`);
		lines.push(`**Sources:** ${concern.researchers.join(", ")}`);
		lines.push("");
		lines.push(concern.description);
		lines.push("");
	}

	// Add validated decisions
	lines.push("## 5. Validated Decisions");
	lines.push("");

	const decisionEntries = Array.from(decisions.entries());
	for (const [questionId, answer] of decisionEntries) {
		lines.push(`### ${questionId}`);
		lines.push("");
		lines.push(`**Decision:** ${answer}`);
		lines.push("");
	}

	// Add deferred decisions
	if (findings.deferredDecisions.length > 0) {
		lines.push("## 6. Open Questions (Deferred)");
		lines.push("");

		for (const decision of findings.deferredDecisions) {
			lines.push(`### ${decision.id}`);
			lines.push("");
			lines.push(`**Question:** ${decision.question}`);
			lines.push(`**Default if unresolved:** ${decision.defaultIfUnresolved}`);
			lines.push("");
		}
	}

	// Add success criteria placeholder
	lines.push("## 7. Success Criteria");
	lines.push("");
	lines.push("- [ ] [To be determined]");
	lines.push("");

	return lines.join("\n");
}

/**
 * Format researcher name for display
 */
function formatResearcherName(name: string): string {
	return name.split("-").map(word => word.charAt(0).toUpperCase() + word.slice(1)).join(" ");
}
