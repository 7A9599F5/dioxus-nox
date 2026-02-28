/**
 * YAML Parser - Simple parser for researcher outputs
 * 
 * Parses structured YAML from researcher sub-agent outputs.
 * Uses basic string parsing - in production, consider using 'js-yaml' library.
 */

export interface ParsedYaml {
	[key: string]: string | number | boolean | null | ParsedYaml | ParsedYaml[];
}

/**
 * Parse YAML string into JavaScript object
 * 
 * This is a simplified parser for the specific YAML format used by researchers.
 * For complex YAML, use a proper library like 'js-yaml'.
 */
export function parseYaml(yaml: string): ParsedYaml {
	const result: ParsedYaml = {};
	const lines = yaml.split("\n");
	
	let currentPath: string[] = [];
	let currentArray: ParsedYaml[] | null = null;
	let arrayKey: string | null = null;

	for (let i = 0; i < lines.length; i++) {
		const line = lines[i];
		const trimmed = line.trimEnd();
		
		// Skip empty lines and comments
		if (trimmed === "" || trimmed.startsWith("#")) continue;

		// Calculate indentation level
		const indent = line.length - line.trimStart().length;
		const level = Math.floor(indent / 2);

		// Adjust current path based on indentation
		while (currentPath.length > level) {
			currentPath.pop();
		}

		const content = trimmed.trim();
		
		// Array item
		if (content.startsWith("- ")) {
			const value = content.slice(2).trim();
			
			if (currentArray === null) {
				// Start new array
				arrayKey = currentPath[currentPath.length - 1];
				currentArray = [];
				setValue(result, currentPath.slice(0, -1), arrayKey!, currentArray);
			}

			if (value.includes(": ")) {
				// Array item with key-value pairs
				const obj: ParsedYaml = {};
				const [key, val] = value.split(": ").map(s => s.trim());
				obj[key] = parseValue(val);
				currentArray.push(obj);
			} else {
				// Simple array item
				currentArray.push(parseValue(value));
			}
			continue;
		}

		// Reset array tracking when we hit a non-array line at same or lower level
		if (currentArray !== null && level <= currentPath.length) {
			currentArray = null;
			arrayKey = null;
		}

		// Key-value pair
		if (content.includes(": ")) {
			const colonIndex = content.indexOf(":");
			const key = content.slice(0, colonIndex).trim();
			const value = content.slice(colonIndex + 1).trim();

			if (value === "" || value === "|" || value === ">") {
				// Multi-line value or block
				currentPath.push(key);
				
				if (value === "|" || value === ">") {
					// Collect multi-line string
					let multiLine = "";
					let j = i + 1;
					const blockIndent = indent + 2;
					
					while (j < lines.length) {
						const nextLine = lines[j];
						const nextIndent = nextLine.length - nextLine.trimStart().length;
						
						if (nextLine.trim() === "" || nextIndent >= blockIndent) {
							multiLine += nextLine.trim() + "\n";
							j++;
						} else {
							break;
						}
					}
					
					i = j - 1;
					setValue(result, currentPath.slice(0, -1), key, multiLine.trim());
					currentPath.pop();
				}
			} else {
				// Simple value
				setValue(result, currentPath, key, parseValue(value));
			}
		} else if (content.endsWith(":")) {
			// Key without value (nested object)
			const key = content.slice(0, -1);
			currentPath.push(key);
		}
	}

	return result;
}

/**
 * Parse a YAML value to appropriate JavaScript type
 */
function parseValue(value: string): string | number | boolean | null {
	// Remove quotes
	if ((value.startsWith('"') && value.endsWith('"')) ||
		(value.startsWith("'") && value.endsWith("'"))) {
		return value.slice(1, -1);
	}

	// Boolean
	if (value === "true") return true;
	if (value === "false") return false;

	// Null
	if (value === "null" || value === "~") return null;

	// Number
	const num = Number(value);
	if (!isNaN(num)) return num;

	// String
	return value;
}

/**
 * Set a value in a nested object using a path
 */
function setValue(obj: ParsedYaml, path: string[], key: string, value: ParsedYaml[keyof ParsedYaml]): void {
	let current = obj;

	for (const segment of path) {
		if (!(segment in current)) {
			current[segment] = {};
		}
		current = current[segment] as ParsedYaml;
	}

	current[key] = value;
}

/**
 * Extract YAML block from markdown text
 */
export function extractYamlBlock(text: string): string | null {
	const match = text.match(/```yaml\n([\s\S]*?)```/);
	return match ? match[1].trim() : null;
}

/**
 * Parse researcher output, extracting YAML if embedded in markdown
 */
export function parseResearcherYaml(text: string): ParsedYaml {
	const yamlBlock = extractYamlBlock(text);
	const yaml = yamlBlock || text;
	return parseYaml(yaml);
}
