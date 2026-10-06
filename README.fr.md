# ProxyIA

[English](README.md) | Français

ProxyIA est un proxy HTTP léger qui intercepte les requêtes envoyées à un backend LLM, applique des remplacements de texte configurables, puis transmet les requêtes modifiées au serveur cible.

Il est utile pour :
- nettoyer ou supprimer des instructions système ou des fragments de prompt
- injecter des paramètres de génération à la demande
- tracer les requêtes reçues et les versions transformées
- relancer automatiquement les requêtes en cas d’erreur ou de réponse vide

## Fonctionnalités

### 1. Proxy HTTP simple

Le service écoute sur une adresse locale configurable et redirige les requêtes vers un backend cible, par exemple :
- llama.cpp
- vLLM
- un autre endpoint compatible OpenAI/Chat Completions

### 2. Remplacements de requêtes

Vous pouvez définir une liste de règles de remplacement dans le fichier de configuration `proxyia.toml`.

Chaque règle supporte :
- `from` : texte à rechercher
- `to` : texte de remplacement
- `first_only` : remplace seulement la première occurrence si `true`
- `from_end` : remplace la dernière occurrence trouvée si `true`

Exemple :

```toml
[[request_replacements]]
from = "Follow Microsoft content policies.\n"
to = ""
first_only = true
from_end = false
```

Le moteur modifie uniquement le corps des requêtes entrantes avant de les envoyer au backend.

### 3. Journalisation

Le proxy peut écrire les requêtes/réponses dans un fichier log.

Options disponibles :
- `log_enabled = true|false`
- `log_file = "proxyia.log"`
- `log_max_bytes = 10485760`
- `log_max_files = 5`
- `max_body_log_bytes = 800000`

Le log contient :
- la requête reçue
- la requête après remplacements, si elle a changé
- les erreurs et retries éventuelles
- les réponses du backend

Les horodatages sont affichés selon le fuseau horaire configuré sur le PC.

### 4. Réessais automatiques

En cas de réponse vide ou d’erreur de connexion, le proxy peut effectuer plusieurs tentatives automatiquement.

Paramètres :
- `max_retries`
- `retry_delay_ms`
- `request_timeout_s`

### 5. Gestion d'un programme externe

ProxyIA peut lancer un programme externe après avoir démarré son serveur, puis le redémarrer périodiquement.

Paramètres dans `proxyia.toml` :
- `managed_program_enabled` : active ou désactive cette fonctionnalité
- `managed_program_path` : chemin de l'exécutable
- `managed_program_args` : liste d'arguments à lui transmettre
- `managed_program_restart_interval_secs` : intervalle de redémarrage en secondes ; `0` signifie démarrage unique

À chaque intervalle, le programme en cours est arrêté puis relancé. Si son démarrage échoue, ProxyIA réessaie au prochain intervalle.

Exemple :

```toml
managed_program_enabled = true
managed_program_path = "C:/Tools/worker.exe"
managed_program_args = ["--serve"]
managed_program_restart_interval_secs = 3600
```

## Fichiers du projet

- `proxyia.toml` : configuration principale
- `build.ps1` : compile la version release
- `run.ps1` : vérifie le binaire puis lance le proxy
- `proxyia.exe` : exécutable final généré à la racine du projet

## Utilisation

### 1. Configurer le proxy

Éditez `proxyia.toml` selon votre environnement :

```toml
listen = "127.0.0.1:8000"
target = "192.168.1.50:8000"
```

- `listen` : adresse où le proxy écoute localement
- `target` : adresse du backend LLM

### 2. Compiler le binaire

Depuis PowerShell :

```powershell
./build.ps1
```

Le binaire est copié dans la racine du projet :

```text
proxyia.exe
```

### 3. Lancer le proxy

```powershell
./run.ps1
```

Le script :
- vérifie que le binaire existe
- le compile si nécessaire
- charge la config `proxyia.toml`
- lance le serveur

## Exemples de configuration

### Nettoyer une instruction système

```toml
[[request_replacements]]
from = "Follow Microsoft content policies.\n"
to = ""
first_only = true
```

### Supprimer une instruction toujours en fin de requête

```toml
[[request_replacements]]
from = "..."
to = ""
first_only = true
from_end = true
```

### Ajouter un paramètre de génération

```toml
[[request_replacements]]
from = "\"max_completion_tokens\":"
to = "\"thinking_token_budget\":50000,\"max_completion_tokens\":"
first_only = true
from_end = true
```

## Remarques

- Les remplacements sont appliqués sur les requêtes entrantes uniquement.
- Les logs peuvent être désactivés avec `log_enabled = false`.
- Le projet est prévu pour être utilisé localement dans un environnement de développement ou de test autour d’un backend LLM.

## Développement

Pour lancer les tests :

```powershell
cargo test -- --nocapture
```

Pour reconstruire le release final :

```powershell
./build.ps1
```
