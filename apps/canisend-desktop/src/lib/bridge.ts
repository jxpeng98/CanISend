import { invoke, isTauri } from "@tauri-apps/api/core";

export interface ProductSummary {
  product: string;
  version: string;
  protocol: string;
  workspace_format: string;
  resource_format: string;
  public_schema_version: string;
  target_os: string;
  target_arch: string;
}

export interface DoctorSummary {
  healthy: boolean;
  embedded_resources: number;
  embedded_renderer: boolean;
  rendered_pages: number;
  render_warning_count: number;
  rendered_pdf_bytes: number;
  render_elapsed_millis: number;
  schema_count: number;
  binary_size_bytes: number;
  release_binary_budget_bytes: number;
  system_font_scan: boolean;
  runtime_package_downloads: boolean;
  python_required: boolean;
}

export interface ActionReceipt<T> {
  operation: string;
  status: string;
  summary: string;
  data: T;
}

export interface DesktopCommandError {
  code: string;
  message: string;
}

export async function getProductSummary(): Promise<ProductSummary> {
  return invoke<ProductSummary>("product_summary");
}

export async function runDoctor(): Promise<ActionReceipt<DoctorSummary>> {
  return invoke<ActionReceipt<DoctorSummary>>("run_doctor");
}

export function isDesktopRuntime(): boolean {
  return isTauri();
}

export function commandErrorMessage(error: unknown): string {
  if (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof error.message === "string"
  ) {
    return error.message;
  }
  if (typeof error === "string") {
    return error;
  }
  return "The desktop command failed without a structured error.";
}
