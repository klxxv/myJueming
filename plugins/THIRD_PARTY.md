# Local research runtime sources

This is resource attribution, not a commercial activation or account dependency.

- XLM-R base: [FacebookAI/xlm-roberta-base](https://huggingface.co/FacebookAI/xlm-roberta-base), immutable revision `e73636d4f797dec63c3081bb6ed5c7b0bb3f2089`. The pinned model card declares MIT. Its original README is retained as `model/README.md` by the release builder.
- CPython 3.12.11: managed relocatable distribution provided through [uv](https://docs.astral.sh/uv/concepts/python-versions/); retain the distribution's license files.
- PyTorch, Transformers, tokenizers, NumPy and transitive dependencies: exact versions are recorded in `requirements-lock.txt`; their wheel metadata and included licenses remain in the packaged `packages/` tree.
- Models and source documents remain on the device during computation. Local workers are trusted executable code; the advertised permissions are capability intent, not an operating-system sandbox.
