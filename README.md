# Kodbot
Det här är botten vi använder på Kodsports Discord-server.

## Konfigurera
Kopiera `config.toml.sample` till `config.toml` och `secrets.toml.sample` till `secrets.toml`. Fyll sedan i fälten enligt instruktionerna. **secrets.toml** får inte publiceras!

## Kör
Efter att konfigurationsfilerna är färdiga kan botten startas genom `cargo run`. Kommandot kommer ladda ned alla paket som behövs och kompilera programmet innan det körs. 

Under körning skapas en `state.toml` som lagrar data som behövs för att få ett konsekvent programtillstånd vid omstart.
