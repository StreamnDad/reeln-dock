import { invoke } from "@tauri-apps/api/core";
import type { PromptTemplateInfo } from "$lib/types/prompt";

export async function listPromptTemplates(): Promise<PromptTemplateInfo[]> {
  return invoke<PromptTemplateInfo[]>("list_prompt_templates");
}

export async function getPromptTemplate(
  name: string,
): Promise<PromptTemplateInfo> {
  return invoke<PromptTemplateInfo>("get_prompt_template", { name });
}

export async function previewPrompt(
  name: string,
  variables: Record<string, string>,
): Promise<string> {
  return invoke<string>("preview_prompt", { name, variables });
}

export async function savePromptTemplate(
  name: string,
  content: string,
): Promise<PromptTemplateInfo> {
  return invoke<PromptTemplateInfo>("save_prompt_template", {
    name,
    content,
  });
}
