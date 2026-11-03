# ForgeRender — NVIDIA Denoising & RTX Integration Study
**Data:** 03/11/2026  
**Destinatario:** Claude Code / ForgeRender maintainers  
**Obiettivo:** individuare la strada più rapida e sicura per integrare denoiser NVIDIA nel rendering finale e nell'anteprima interattiva.

> **Limite importante dell'audit:** in questa sessione GitHub ha mostrato i branch `main` e `sviluppo`, ma il recupero dei file è fallito, inclusi `reports/chatgpt/CLAUDE_HANDOFF.md`, `Cargo.toml` e i manifest dei crate. Non sono quindi verificati HEAD, ultima risposta Claude, API grafica, denoiser attuale, percorso CUDA/OptiX e buffer GPU. Claude deve prima leggere l'handoff aggiornato e verificare il codice reale. Non assumere che ForgeRender sia Vulkan, wgpu o CUDA-native sulla base di questo report.

## 1. Raccomandazione esecutiva

Non sostituire subito il denoiser attuale. Aggiungere un backend selezionabile e confrontare con input identici:

1. **OptiX Denoiser** — primo candidato per rendering statico HDR/AOV se il renderer possiede già buffer CUDA e stream CUDA.
2. **NVIDIA NRD** — primo candidato se ForgeRender è Vulkan-native e può esporre immagini, device e command buffer Vulkan; valutare anche un bridge RHI/NRI.
3. **Denoiser ForgeRender attuale** — mantenerlo come baseline e fallback.

**DLSS Ray Reconstruction** è un percorso avanzato di ricostruzione temporale, non un drop-in universale per immagini finali statiche. **ReSTIR PT** e **SER** sono ottimizzazioni del path tracer, non denoiser.

La prima milestone deve essere un singolo backend esterno, con test A/B riproducibile, senza riscrivere contemporaneamente sampling, temporal accumulation e denoising.

## 2. Audit obbligatorio del repository

Prima di modificare codice Claude deve:
- leggere `reports/chatgpt/CLAUDE_HANDOFF.md`;
- leggere `reports/CLAUDE_HANDOFF.md`, se esiste, e i report citati;
- registrare branch, HEAD SHA, data/ultima risposta Claude;
- confrontare `main` e `sviluppo` e individuare il branch effettivamente più aggiornato.

### 2.1 Architettura GPU/API
Individuare con percorsi e simboli:
- renderer attivo: CUDA, OptiX, Vulkan RT, wgpu, ibrido o multiplo;
- chi possiede beauty e guide al momento del denoising;
- se le immagini sono in CUDA device memory, Vulkan image, wgpu texture, RAM host o copiate tra API;
- gestione di device/context/stream CUDA, queue/command buffer Vulkan e sincronizzazione;
- eventuale interop CUDA–Vulkan (external memory + semaphores);
- formati, stride, allineamento, color space e transizioni di stato;
- capability detection e fallback GPU.

### 2.2 Denoiser esistente
Individuare:
- crate/modulo e implementazione;
- formati e precisione degli input/output;
- gestione HDR/linear e tone mapping;
- disponibilità di albedo e normali pulite;
- separazione diffuse/specular;
- history temporale, motion vectors, reprojection e reset;
- readback CPU/GPU e sincronizzazione;
- impostazioni UI e default.

### 2.3 AOV/G-buffer disponibili
Verificare se il path tracer produce:
- noisy beauty HDR lineare;
- diffuse albedo pulita;
- primary-hit shading normal pulita;
- world/view position o linear view depth;
- roughness lineare;
- diffuse e specular radiance separate;
- hit distance / first-hit distance, dove necessario;
- motion vectors con camera e oggetti dinamici;
- object/material ID per diagnostica e validazione history.

Non usare come guide dati display-space, gamma-encoded, normal map o base color arbitrari. Ogni guida deve rispettare definizione e coordinate richieste dallo specifico SDK.

## 3. OptiX AI Denoiser

### Documentazione e codice di riferimento
- SDK/download/release notes: https://developer.nvidia.com/designworks/optix/download
- API denoiser: https://raytracing-docs.nvidia.com/optix9/api/group__optix__host__api__denoiser.html
- Tipi e model kinds: https://raytracing-docs.nvidia.com/optix9/api/group__optix__types.html
- Programming Guide: https://raytracing-docs.nvidia.com/optix9/guide/optix_guide.250130.A4.pdf
- Esempi avanzati: https://github.com/NVIDIA/OptiX_Apps
- Esempio denoiser: https://github.com/NVIDIA/OptiX_Apps/tree/master/apps/intro_denoiser

### Model kinds importanti
- `OPTIX_DENOISER_MODEL_KIND_HDR`: singola immagine HDR.
- `OPTIX_DENOISER_MODEL_KIND_AOV`: immagine HDR con più AOV.
- `OPTIX_DENOISER_MODEL_KIND_TEMPORAL_AOV`: sequenza temporalmente stabile con AOV.
- `OPTIX_DENOISER_MODEL_KIND_UPSCALE2X`: denoise e upscaling 2× per singolo frame.
- `OPTIX_DENOISER_MODEL_KIND_TEMPORAL_UPSCALE2X`: variante temporale + upscaling 2×.
- I vecchi LDR/TEMPORAL sono indicati come alias deprecati nelle API recenti: preferire le varianti AOV quando adatte.

### Sequenza di integrazione
1. Creare `OptixDeviceContext` sul contesto CUDA che possiede i buffer.
2. Configurare `OptixDenoiserOptions` per guide albedo/normali.
3. `optixDenoiserCreate`.
4. `optixDenoiserComputeMemoryResources` alle dimensioni effettive.
5. Allocare state e scratch GPU con le dimensioni restituite.
6. `optixDenoiserSetup` con dimensioni e CUDA stream.
7. Preparare `OptixDenoiserGuideLayer` e uno o più `OptixDenoiserLayer`.
8. Configurare `OptixDenoiserParams` (HDR intensity/average color; history se temporale).
9. Invocare `optixDenoiserInvoke` sullo stream CUDA correttamente sincronizzato.
10. Mantenere i buffer vivi fino al completamento GPU; gestire resize e shutdown.

### Rischi/compatibilità
- L'API host usa `CUdeviceptr` e CUDA stream: è ideale se ForgeRender produce già i buffer in memoria CUDA.
- Se il renderer è wgpu/Vulkan, OptiX non è un denoiser Vulkan plug-and-play. Serve interop external memory/semaphore, compatibilità formati, ownership e sincronizzazione oppure copie esplicite. Misurare costo e complessità prima di scegliere.
- Per immagini finali statiche partire da HDR/AOV, non temporal.
- 2× upscale va testato separatamente: cambia dimensioni e caratteristiche dell'immagine.
- Temporal AOV richiede flow/motion vector corretti e doppio buffering di output e internal guide layers. Resettare history su camera cut, cambi scena, resize e invalidazioni.
- Input beauty lineare/HDR: niente tone mapping o gamma prima del denoising.
- Testare metalli glossy, specular, vetro/refraction, caustiche, geometrie sottili, emissivi, texture ad alta frequenza e volumi. Non misurare solo la “pulizia”: controllare perdita di dettaglio e bias.

La pagina OptiX 9.1 consultata indica Windows/Linux/Arm e requisito driver R590 o successivo. È un requisito legato a quella release: verificare versione SDK e driver compatibili con il target Windows e le GPU supportate prima di fissare la dipendenza.

## 4. NVIDIA NRD

### Documentazione e sample
- SDK: https://github.com/NVIDIA-RTX/NRD
- Sample completo: https://github.com/NVIDIA-RTX/NRD-Sample
- Esempio d'integrazione Vulkan: https://github.com/nvpro-samples/vk_denoise_nrd
- Developer page: https://developer.nvidia.com/nvidia-rt-denoiser

NRD è particolarmente interessante per renderer Vulkan-native. La libreria non effettua direttamente chiamate GAPI: l'applicazione gestisce allocazione risorse, sincronizzazione e dispatch compute. È disponibile integrazione tramite NRI oppure implementazione nativa Vulkan.

### Denoiser
- **REBLUR:** recurrent blur; diffuse/specular radiance e alcuni segnali occlusion. Primo candidato da provare su un path tracer convenzionale.
- **RELAX:** A-trous, pensato per segnali in stile RTXDI; diffuse/specular radiance. Da confrontare con REBLUR, non presumere sia sempre migliore.
- **SIGMA:** denoiser specializzato per ombre.

### Flusso d'integrazione
1. Scegliere i denoiser e creare istanza NRD.
2. Interrogare library/instance descriptions: pipeline, sampler, texture pool, constant buffer e descriptor/binding.
3. Allocare texture permanenti e transient nell'engine. NRD descrive le risorse ma non le alloca.
4. Ogni frame chiamare `SetCommonSettings` e `SetDenoiserSettings`.
5. Popolare `ResourceSnapshot` con input/output: motion vectors, normal/roughness, viewZ, radiance rumorosa diffuse/specular e output denoised.
6. Chiamare `GetComputeDispatches` e registrare i dispatch compute nel command buffer del renderer.
7. Gestire resource state transitions e ripristinare descriptor/pipeline state di ForgeRender se necessario.
8. Gestire history e reset; su resize distruggere/ricreare istanza e risorse (NRD non offre un semplice resize in-place).

### Requisiti e insidie
- Roughness lineare e normali world-space.
- Motion vectors non jittered; convenzione documentata `old = new + MV`. Preferire 2.5D screen-space motion ove possibile, includendo previous view depth.
- ViewZ lineare e guide valide; niente NaN/Inf nei guide input.
- Material demodulation: NRD non usa direttamente baseColor/metalness. Rimuovere i fattori materiale dalla radiance rumorosa prima del denoising e ripristinarli dopo, seguendo gli helper del sample.
- NRD non è nativamente progettato per volumetrici o trasparenze. Non aspettarsi che i denoiser di superfici risolvano automaticamente fumo/fog/volume. Mantenere un percorso separato o il fallback nativo.
- La modalità SH (spherical Gaussian) è descritta da NVIDIA come comparabile a DLSS-RR; non è garanzia per ogni scena.
- README NRD riporta su RTX 4080 @ 1440p: REBLUR diffuse/specular 2.55 ms (3.40 ms SH), RELAX 3.25 ms (4.80 ms SH), SIGMA shadow 0.40 ms. Sono numeri del sample/vendor, non previsioni ForgeRender e non confrontabili direttamente con OptiX.

**Percorso rapido consigliato:** se ForgeRender è Vulkan, partire da `vk_denoise_nrd`; poi esaminare branch `simplex` di NRD-Sample per integrazione best-practice/minimal, quindi `main` per opzioni avanzate. Usare NRD debug durante lo sviluppo.

## 5. DLSS Ray Reconstruction

- Streamline SDK: https://github.com/NVIDIA-RTX/Streamline
- Programming Guide: https://github.com/NVIDIA-RTX/Streamline/blob/main/docs/ProgrammingGuideDLSS_RR.md
- Integration Guide PDF: https://github.com/NVIDIA-RTX/Streamline/blob/main/docs/DLSS-RR%20Integration%20Guide.pdf

DLSS-RR non è una semplice API denoiser che riceve solo beauty rumorosa. Il guide Streamline richiede corretti input tra cui:
- noisy ray-traced color;
- diffuse albedo;
- specular albedo;
- shading normals normalizzate;
- roughness lineare;
- dense motion vectors (camera + oggetti dinamici);
- depth e metadata frame/dispatch richiesti da Streamline.

Raccomandazione: rimandare DLSS-RR finché AOV/G-buffer e motion vectors non sono corretti. Prima un denoising single-frame stabile; poi prototipo DLSS-RR come percorso temporale opzionale. Verificare supporto API grafica, deployment plugin, requisiti driver/GPU e licenze. Non dedurre che CUDA interop implichi automaticamente supporto DLSS-RR.

## 6. ReSTIR PT / RTXDI

- SDK: https://github.com/NVIDIA-RTX/RTXDI
- ReSTIR PT integration: https://github.com/NVIDIA-RTX/RTXDI/blob/main/Doc/RestirPT.md
- General integration: https://github.com/NVIDIA-RTX/RTXDI/blob/main/Doc/Integration.md

Non è un denoiser. Usa reservoir e resampling spaziale/temporale per riutilizzare percorsi di luce e aumentare il numero effettivo di campioni. Richiede integrazione profonda nel path tracer: `RAB_PathTrace`, callback `RTXDI_PathTracerContext`, motion vectors, surface validation, shift mapping e reservoir GPU. La documentazione dice esplicitamente che l'integrazione è significativa e dipende da material model, scene representation, ray tracing, G-buffer e graphics API.

Rimandare a un progetto successivo: non è una sostituzione rapida del denoiser.

## 7. Shader Execution Reordering (SER)

- Guida/sample Vulkan: https://docs.vulkan.org/samples/latest/samples/extensions/ray_tracing_invocation_reorder/README.html

SER riduce divergenza shader e accessi incoerenti nel ray tracing. Può aiutare path tracing multi-bounce, materiali differenti e raggi secondari incoerenti. Non denoisa e non garantisce vantaggi in tutte le scene. Il sample Vulkan richiede `VK_EXT_ray_tracing_invocation_reorder`; il sample corrente raccomanda Slang perché il supporto glslc può essere incompleto. Fare feature query e mantenere fallback. Se ForgeRender usa CUDA/OptiX invece di Vulkan RT, studiare capacità specifiche OptiX: non portare direttamente il codice SER Vulkan.

## 8. Matrice decisionale

| Architettura verificata | Primo candidato | Motivazione |
|---|---|---|
| Renderer possiede buffer CUDA e stream CUDA | OptiX HDR/AOV | Accesso diretto a device pointer/stream |
| Renderer Vulkan-native | NRD via NRI o Vulkan nativo | Integrazione compute Vulkan, evita CUDA interop |
| Renderer wgpu senza native Vulkan handles esposti | Audit RHI/native handles prima | Evitare readback CPU e copie nascoste |
| Final still render | Confronto OptiX AOV vs NRD REBLUR/RELAX | A/B misurabile |
| Preview/animazione temporale | NRD temporal o OptiX TEMPORAL_AOV; poi DLSS-RR | Richiede motion vector e history corretti |
| Accelerare illuminazione indiretta | RTXDI ReSTIR PT | Integrazione profonda path tracer |
| Ridurre divergenza shader | SER capability prototype | Dipende da API/GPU |

Non descrivere OptiX, NRD, DLSS-RR, ReSTIR e SER come tecnologie intercambiabili.

## 9. Architettura consigliata

Introdurre un'interfaccia neutrale rispetto al backend, adattando nomi e collocazione ai crate già esistenti (non duplicare il render loop):

```text
DenoiserBackend
  initialize(device, capabilities, resolution)
  required_guides() -> GuideRequirements
  denoise(frame_inputs, frame_context) -> output
  resize(resolution)
  reset_history(reason)
  shutdown()

Implementations:
  NativeDenoiserBackend
  OptixDenoiserBackend
  NrdDenoiserBackend
  DlssRayReconstructionBackend  // in seguito, opzionale
```

Mantenere risorse GPU residenti. Non copiare full-res su CPU per consegnarle a un SDK. Documentare per backend: GPU/API/driver supportati, formati guide, ownership/sync, memoria, resize/reset, fallback, licenza e deployment. La disponibilità SDK NVIDIA non deve diventare requisito per avviare ForgeRender.

### Contratto AOV minimo
- Beauty: noisy, linear HDR RGB.
- Albedo: clean linear diffuse albedo.
- Normal: clean normalized primary-hit normal, coordinate space esplicito.
- Roughness: linear roughness.
- Depth/viewZ: linear depth, segno/unità documentati.
- Motion vectors: solo per temporal; convenzione pixel/UV, jitter, camera e object motion documentati.
- Diffuse/specular radiance AOV opzionali ma raccomandati per NRD/DLSS/RTXDI avanzati.

Ogni buffer deve avere descriptor tipizzato: dimensioni, formato, color space, coordinate space, row stride, sample count, valid rect e lifetime.

## 10. Proof of concept più rapido e sicuro

### Fase A — baseline
1. Scena fissa con diffuse, metallo, glossy rough, speculare quasi mirror, vetro/refraction, geometrie sottili, emissivi, caustiche e volume.
2. Render reference ad alto SPP.
3. Salvare input rumoroso e output denoiser corrente a budget fissi (es. 1, 2, 4, 8, 16, 32 spp, secondo renderer).
4. Validare albedo e normal guide.
5. Misurare render time, denoise time, total time, VRAM e qualità.

### Fase B — un solo SDK
- Se CUDA è già la memoria nativa di output: OptiX HDR come primo PoC usando `intro_denoiser`.
- Se Vulkan è nativo: NRD usando `vk_denoise_nrd`, poi validare con NRD-Sample.
- Non creare interop CUDA/Vulkan solo per evitare questa decisione: quantificare prima se NRD è integrabile direttamente.
- Conservare denoiser corrente e aggiungere selettore runtime/debug.

### Fase C — confronto qualità
Confrontare denoiser nativo, OptiX HDR/AOV, NRD REBLUR e NRD RELAX/SH ove supportato, usando stesso noisy input, risoluzione, SPP, exposure e tonemapping. Salvare immagini e metriche:
- PSNR/SSIM e, se disponibile, metrica percettiva LPIPS contro reference high-SPP;
- crop di bordi/dettagli;
- specular/reflection e vetro;
- caustiche e volumi;
- firefly handling;
- stabilità temporale in test separato con camera in movimento.

Non dichiarare migliore un denoiser solo perché l'immagine è più liscia: misurare perdita di dettaglio, bias, ghosting e piccoli highlight persi.

### Fase D — policy final render
Solo dopo A/B:
- scegliere backend predefinito per classe di segnale/scena;
- mantenere override utente e fallback nativo;
- esporre opzioni qualità solo se il loro significato è compreso;
- consentire più pass/AOV se necessario;
- registrare backend/versione/settings nei metadata render.

## 11. Criteri di accettazione

Nessuna promessa di performance/qualità senza misure sull'hardware target dell'utente (storicamente indicato Ryzen 9 9900X + RTX 5070: verificare GPU e driver reali).

Per accettare un backend nel final render:
- nessun crash/errore sync dopo render ripetuti e resize;
- nessun degrado inaccettabile rispetto alla reference high-SPP;
- miglioramento dimostrato su più classi di materiali;
- VRAM e latency denoise misurate;
- riproducibilità entro variazioni floating point attese;
- fallback verificato se SDK/GPU feature non disponibile.

Per modalità temporali: motion vectors camera/oggetti corretti, reset history su camera cut/scene edits/resize/invalidation, nessun ghosting persistente.

## 12. Licenze e packaging

Prima di distribuire:
- leggere licenza/SDK agreement della versione esatta di OptiX, NRD, Streamline/DLSS e RTXDI;
- distinguere source availability da permessi di redistribuzione di binari/plugin;
- stabilire requisiti driver NVIDIA;
- distribuire DLL/plugin secondo i termini SDK;
- capability detection e messaggio fallback;
- non includere sample assets/binari terzi senza controllo licenza.

## 13. Link diretti prioritari per Claude

1. OptiX denoiser sample: https://github.com/NVIDIA/OptiX_Apps/tree/master/apps/intro_denoiser
2. OptiX API: https://raytracing-docs.nvidia.com/optix9/api/group__optix__host__api__denoiser.html
3. NRD SDK/integration: https://github.com/NVIDIA-RTX/NRD
4. NRD Vulkan sample: https://github.com/nvpro-samples/vk_denoise_nrd
5. NRD Sample simplex branch: https://github.com/NVIDIA-RTX/NRD-Sample/tree/simplex
6. DLSS-RR guide: https://github.com/NVIDIA-RTX/Streamline/blob/main/docs/ProgrammingGuideDLSS_RR.md
7. RTXDI ReSTIR PT: https://github.com/NVIDIA-RTX/RTXDI/blob/main/Doc/RestirPT.md
8. Vulkan SER: https://docs.vulkan.org/samples/latest/samples/extensions/ray_tracing_invocation_reorder/README.html

## 14. Prompt operativo per Claude Code

> Leggi prima questo report e gli handoff ForgeRender aggiornati. Verifica branch e commit correnti; mappa architettura renderer, API grafica, uso CUDA/OptiX, denoiser, ownership buffer e AOV già disponibili. Non assumere che ForgeRender usi Vulkan né che i buffer CUDA siano accessibili direttamente da OptiX. Non modificare ancora codice. Restituisci un audit basato su evidenze con file path e simboli, indica se OptiX o NRD è la via più rapida per l'architettura reale, elenca AOV mancanti, lavoro di sincronizzazione, licenze/deployment e un PoC minimo con file da toccare. Conserva il denoiser attuale come fallback. Attendi approvazione prima di implementare.

## 15. Provenienza e limiti

Questo documento combina documentazione ufficiale NVIDIA/Khronos e sample repository con raccomandazioni ingegneristiche. I numeri prestazionali NRD citati sono dati vendor/sample, non benchmark indipendenti. Non sono stati eseguiti build, test, benchmark o ispezione del codice ForgeRender durante questa ricerca perché il recupero GitHub dei file è fallito. La scelta definitiva deve seguire l'audit del codice.

Fonti ufficiali:
- OptiX SDK: https://developer.nvidia.com/designworks/optix/download
- OptiX API: https://raytracing-docs.nvidia.com/optix9/api/group__optix__host__api__denoiser.html
- OptiX model kinds: https://raytracing-docs.nvidia.com/optix9/api/group__optix__types.html
- OptiX Programming Guide: https://raytracing-docs.nvidia.com/optix9/guide/optix_guide.250130.A4.pdf
- OptiX samples: https://github.com/NVIDIA/OptiX_Apps
- NRD: https://github.com/NVIDIA-RTX/NRD
- NRD-Sample: https://github.com/NVIDIA-RTX/NRD-Sample
- NRD Vulkan integration: https://github.com/nvpro-samples/vk_denoise_nrd
- Streamline DLSS-RR: https://github.com/NVIDIA-RTX/Streamline/blob/main/docs/ProgrammingGuideDLSS_RR.md
- RTXDI ReSTIR PT: https://github.com/NVIDIA-RTX/RTXDI/blob/main/Doc/RestirPT.md
- Vulkan SER: https://docs.vulkan.org/samples/latest/samples/extensions/ray_tracing_invocation_reorder/README.html
