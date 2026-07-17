declare module "rscpp-wasm" {
  const init: (module_or_path?: unknown) => Promise<unknown>;
  export default init;
  export function run(source: string): unknown;
  export function run_method(source: string, method: string, args: unknown): unknown;
}
