/**
 * Researcher Dispatch - Shared utility for spawning researcher sub-agents
 * 
 * Used by:
 * - Planning extension (initial dispatch)
 * - Implementation extension (re-consult on triggers)
 */

import { spawn } from "node:child_process";
import * as path from "node:path";
import * as fs from "node:fs";

export interface ResearcherConfig {
	name: string;
	description: string;
	triggerSignals: string[];
	always: boolean;
	filePath: string;
}

export interface ResearcherOutput {
	researcher: string;
	domain_summary: string;
	concerns: ResearcherConcern[];
	workflow_guidance: WorkflowGuidance;
	raw: string;
}

export interface ResearcherConcern {
	id: string;
	severity: "critical" | "high" | "medium" | "low";
	category: string;
	description: string;
	why_it_matters: string;
	default_assumption: string;
	questions: ResearcherQuestion[];
}

export interface ResearcherQuestion {
	question: string;
	options: QuestionOption[];
	hybrid_possible: boolean;
}

export interface QuestionOption {
	label: string;
	tradeoffs: string;
}

export interface WorkflowGuidance {
	phase: "foundation" | "core" | "integration" | "polish";
	order: number;
	tasks: string[];
	checkpoint_questions: string[];
	reconsult_when: string[];
	testing_milestones: string[];
}

export interface DispatchOptions {
	cwd: string;
	model?: string;
	maxConcurrent?: number;
	signal?: AbortSignal;
	onProgress?: (name: string, status: "started" | "complete" | "error", output?: string) => void;
}

// Researcher registry with trigger signals
const RESEARCHER_REGISTRY: Array<{ name: string; triggerSignals: string[]; always: boolean }> = [
	{ name: "security", triggerSignals: ["auth", "login", "password", "session", "token", "encrypt", "credential", "user data"], always: false },
	{ name: "ux-ui", triggerSignals: ["form", "button", "modal", "navigation", "dashboard", "UI", "UX", "user flow", "interface"], always: false },
	{ name: "performance", triggerSignals: ["scale", "thousands", "real-time", "latency", "slow", "fast", "cache", "optimize"], always: false },
	{ name: "data-database", triggerSignals: ["database", "store", "query", "table", "model", "schema", "migration", "data"], always: false },
	{ name: "api", triggerSignals: ["API", "endpoint", "REST", "GraphQL", "webhook", "HTTP", "request"], always: false },
	{ name: "accessibility", triggerSignals: ["form", "button", "UI", "navigation", "interface", "a11y", "accessibility"], always: false },
	{ name: "testing", triggerSignals: [], always: true }, // Always dispatch
	{ name: "architecture", triggerSignals: ["module", "package", "dependency", "architecture", "structure", "refactor"], always: false },
	{ name: "devops", triggerSignals: ["deploy", "production", "staging", "CI/CD", "infrastructure", "Docker", "Kubernetes"], always: false },
	{ name: "compliance", triggerSignals: ["HIPAA", "GDPR", "SOC2", "compliance", "audit", "PII", "PHI", "regulation"], always: false },
];

/**
 * Discover available researchers from .pi/agents/ directory
 */
export function discoverResearchers(cwd: string): ResearcherConfig[] {
	const agentsDir = path.join(cwd, ".pi", "agents");
	const researchers: ResearcherConfig[] = [];

	if (!fs.existsSync(agentsDir)) {
		return researchers;
	}

	const entries = fs.readdirSync(agentsDir, { withFileTypes: true });
	
	for (const entry of entries) {
		if (!entry.name.endsWith(".md")) continue;
		if (!entry.isFile() && !entry.isSymbolicLink()) continue;

		const filePath = path.join(agentsDir, entry.name);
		const content = fs.readFileSync(filePath, "utf-8");
		
		// Parse frontmatter
		const frontmatterMatch = content.match(/^---\n([\s\S]*?)\n---/);
		if (!frontmatterMatch) continue;

		const frontmatterLines = frontmatterMatch[1].split("\n");
		const frontmatter: Record<string, string> = {};
		
		for (const line of frontmatterLines) {
			const [key, ...valueParts] = line.split(":");
			if (key && valueParts.length > 0) {
				frontmatter[key.trim()] = valueParts.join(":").trim();
			}
		}

		if (!frontmatter.name || !frontmatter.description) continue;

		const registryEntry = RESEARCHER_REGISTRY.find(r => r.name === frontmatter.name);
		
		researchers.push({
			name: frontmatter.name,
			description: frontmatter.description,
			triggerSignals: registryEntry?.triggerSignals || [],
			always: registryEntry?.always || false,
			filePath,
		});
	}

	return researchers;
}

/**
 * Match researchers to dispatch based on query analysis
 */
export function matchResearchers(query: string, researchers: ResearcherConfig[]): string[] {
	const queryLower = query.toLowerCase();
	const matched = new Set<string>();

	for (const researcher of researchers) {
		// Always-on researchers
		if (researcher.always) {
			matched.add(researcher.name);
			continue;
		}

		// Signal-based matching
		for (const signal of researcher.triggerSignals) {
			if (queryLower.includes(signal.toLowerCase())) {
				matched.add(researcher.name);
				break;
			}
		}
	}

	return Array.from(matched);
}

/**
 * Dispatch a single researcher as a sub-agent process
 */
export async function dispatchResearcher(
	researcherName: string,
	query: string,
	context: string,
	options: DispatchOptions
): Promise<ResearcherOutput> {
	const { cwd, model = "glm-5", signal, onProgress } = options;

	onProgress?.(researcherName, "started");

	// Build the prompt for the researcher
	const agentPath = path.join(cwd, ".pi", "agents", `${researcherName}.md`);
	const agentContent = fs.readFileSync(agentPath, "utf-8");

	// Extract system prompt (after frontmatter)
	const systemPrompt = agentContent.replace(/^---\n[\s\S]*?\n---\n/, "");

	const fullPrompt = `${systemPrompt}

## Context

${context}

## Query

${query}

## Instructions

Analyze this request from your domain perspective and return your findings in the YAML format specified above.`;

	// Spawn pi process with the researcher prompt
	const result = await runPiSubprocess(fullPrompt, { cwd, model, signal });

	if (result.error) {
		onProgress?.(researcherName, "error", result.error);
		throw new Error(`Researcher ${researcherName} failed: ${result.error}`);
	}

	onProgress?.(researcherName, "complete", result.output);

	// Parse the YAML output
	const output = parseResearcherOutput(researcherName, result.output);
	return output;
}

/**
 * Dispatch multiple researchers in parallel
 */
export async function dispatchResearchers(
	researcherNames: string[],
	query: string,
	context: string,
	options: DispatchOptions
): Promise<Map<string, ResearcherOutput>> {
	const { maxConcurrent = 4 } = options;
	const results = new Map<string, ResearcherOutput>();
	const errors: Error[] = [];

	// Process in batches
	for (let i = 0; i < researcherNames.length; i += maxConcurrent) {
		const batch = researcherNames.slice(i, i + maxConcurrent);
		
		const batchPromises = batch.map(async (name) => {
			try {
				const output = await dispatchResearcher(name, query, context, options);
				return { name, output, error: null };
			} catch (err) {
				return { name, output: null, error: err as Error };
			}
		});

		const batchResults = await Promise.all(batchPromises);
		
		for (const { name, output, error } of batchResults) {
			if (error) {
				errors.push(error);
			} else if (output) {
				results.set(name, output);
			}
		}
	}

	if (errors.length > 0 && results.size === 0) {
		throw new AggregateError(errors, "All researchers failed");
	}

	return results;
}

/**
 * Run pi as a subprocess
 */
async function runPiSubprocess(
	prompt: string,
	options: { cwd: string; model: string; signal?: AbortSignal }
): Promise<{ output: string; error: string | null }> {
	return new Promise((resolve) => {
		const args = [
			"--model", options.model,
			"--no-skills",
			"--no-extensions",
			"--print",
			prompt,
		];

		const proc = spawn("pi", args, {
			cwd: options.cwd,
			signal: options.signal,
		});

		let stdout = "";
		let stderr = "";

		proc.stdout.on("data", (data) => {
			stdout += data.toString();
		});

		proc.stderr.on("data", (data) => {
			stderr += data.toString();
		});

		proc.on("close", (code) => {
			if (code !== 0) {
				resolve({ output: stdout, error: stderr || `Process exited with code ${code}` });
			} else {
				resolve({ output: stdout, error: null });
			}
		});

		proc.on("error", (err) => {
			resolve({ output: "", error: err.message });
		});
	});
}

/**
 * Parse researcher YAML output into structured format
 */
function parseResearcherOutput(researcherName: string, raw: string): ResearcherOutput {
	// Extract YAML block from output
	const yamlMatch = raw.match(/```yaml\n([\s\S]*?)```/);
	const yamlContent = yamlMatch ? yamlMatch[1] : raw;

	// Simple YAML parsing (for structured researcher output)
	// In production, use a proper YAML library
	const lines = yamlContent.split("\n");
	
	const output: ResearcherOutput = {
		researcher: researcherName,
		domain_summary: "",
		concerns: [],
		workflow_guidance: {
			phase: "core",
			order: 1,
			tasks: [],
			checkpoint_questions: [],
			reconsult_when: [],
			testing_milestones: [],
		},
		raw,
	};

	// Basic parsing - extract key fields
	let currentSection = "";
	let currentConcern: Partial<ResearcherConcern> | null = null;

	for (const line of lines) {
		const trimmed = line.trim();
		
		if (trimmed.startsWith("domain_summary:")) {
			output.domain_summary = trimmed.replace("domain_summary:", "").trim().replace(/"/g, "");
		}
		
		if (trimmed === "concerns:") {
			currentSection = "concerns";
		} else if (trimmed === "workflow_guidance:") {
			currentSection = "workflow";
		}

		if (currentSection === "workflow") {
			if (trimmed.startsWith("phase:")) {
				const phase = trimmed.replace("phase:", "").trim() as WorkflowGuidance["phase"];
				if (["foundation", "core", "integration", "polish"].includes(phase)) {
					output.workflow_guidance.phase = phase;
				}
			}
			if (trimmed.startsWith("order:")) {
				output.workflow_guidance.order = parseInt(trimmed.replace("order:", "").trim(), 10) || 1;
			}
			if (trimmed.startsWith("- ") && line.includes("tasks:")) {
				// Task list items
			}
		}
	}

	return output;
}

export { RESEARCHER_REGISTRY };
