/**
 * Which NVIDIA-hosted model generates the GUML.
 *
 * These choices are measured, not guessed. With the real 11k-character system prompt, over
 * a free build.nvidia.com key:
 *
 * | model                                    | first token | note                        |
 * |------------------------------------------|-------------|-----------------------------|
 * | `meta/llama-3.1-8b-instruct`             | 488 ms      | valid GUML immediately      |
 * | `meta/llama-3.3-70b-instruct`            | 212 s       | unusable interactively      |
 * | `openai/gpt-oss-120b`                    | >90 s       | timed out                   |
 * | `nvidia/llama-3.3-nemotron-super-49b`    | —           | 14 s, emitted no content    |
 * | `google/gemma-3-12b-it`                  | —           | listed, returns 404          |
 * | `deepseek-ai/deepseek-v4-flash`          | —           | listed, `DEGRADED`          |
 * | `meta/llama-3.1-70b-instruct`, mistral,  | —           | listed, 400/404             |
 * | codestral, granite, mistral-nemo         |             |                             |
 *
 * So `GET /v1/models` lists what the account can *see*, not what it can *call*: 102 ids, of
 * which one worked. The picker therefore separates verified models from the catalogue rather
 * than presenting 102 equally-plausible options, most of which fail.
 *
 * The default is deliberately the small model. GUML generation is a formatting-discipline
 * task — closed vocabulary, significant indentation, no prose around the output — and an 8B
 * model produced correct output in a second. That is the project's own thesis showing up in
 * its own product: a cheap model plus a compiler that owns the hard parts.
 */

/**
 * Hosted NIM by default. Overridable because NVIDIA also ships NIMs you run yourself, and
 * because a local stub is how the rate limiter gets tested without spending real credits.
 */
export const NVIDIA_BASE = process.env.NVIDIA_BASE_URL || "https://integrate.api.nvidia.com/v1";

export const DEFAULT_MODEL = "meta/llama-3.1-8b-instruct";

/** Confirmed callable, and fast enough to sit behind a text box. */
export const VERIFIED_MODELS = ["meta/llama-3.1-8b-instruct"];

/** Callable but too slow for interactive use; offered with a warning. */
export const SLOW_MODELS = ["meta/llama-3.3-70b-instruct", "openai/gpt-oss-120b"];
