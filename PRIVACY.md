# Privacy

Architekton is a tool that runs on your computer. It reads the folders you open so it can draw their architecture, and that is all it does with them.

- **Your code stays on your computer.** Architekton does not upload, share or send your code, file names, analysis results or usage anywhere. There is no account, no analytics and no telemetry.
- **Update check.** A few seconds after launch and every six hours, Architekton downloads a small public file (`latest.json`) from GitHub Releases to see whether a newer version exists. It sends nothing about you or your code; GitHub sees an ordinary download request. Updates are only installed when you click Update, and the app refuses any package whose signature does not match.
- **What is stored locally.** To make the next analysis faster, Architekton keeps a cache of parsed results in your user cache folder (for example `%LOCALAPPDATA%\architekton` on Windows, `~/Library/Caches/architekton` on macOS, `~/.cache/architekton` on Linux). Delete that folder at any time; nothing is lost except speed. Your theme choice is kept in the app's local storage.
- **Optional AI models.** If you choose to install the local AI components, Architekton downloads them once from their official sources (GitHub for llama.cpp, Hugging Face for the models), checks each file's SHA-256 checksum, and stores them in the app's data folder. The models then run on your computer, listening only on `127.0.0.1`. Nothing you analyse is sent to those websites or to anyone else. Remove the components from the AI setup screen to delete them.
- **External model servers.** If you already run Ollama, llama.cpp or LM Studio, Architekton can use it, but only on `127.0.0.1` (this computer).

Questions or concerns: open an issue at https://github.com/IamSamk/Architekton/issues.
