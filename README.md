# 🐾 Desktop Pet Zero

Um pet de desktop animado que anda pela sua tela com animações fluidas. Escolha entre **3300+ pets** do [Petdex](https://petdex.dev) ou use os pets instalados do Hermes.

![Python](https://img.shields.io/badge/Python-3.11+-blue?logo=python&logoColor=white)
![Platform](https://img.shields.io/badge/Platform-Windows-0078d4?logo=windows&logoColor=white)
![License](https://img.shields.io/badge/License-MIT-green)

---

## ✨ Funcionalidades

- **🐾 3300+ pets** — baixe qualquer pet do Petdex direto pelo menu
- **🏃 Roam** — o pet anda pela tela automaticamente com pausas e animações
- **⬇️ Gravidade** — solte o pet no ar e ele cai com animação de jump
- **🖱️ Arrastável** — clique e arraste o pet pra qualquer lugar da tela
- **🔍 Busca** — encontre pets por nome ou slug
- **⚙️ Configurações** — tamanho, ativar/desativar, roaming
- **🎨 Dark theme** — interface moderna e escura
- **🚀 Tray icon** — minimize pra bandeja do Windows

---

## 📦 Instalação

### Pré-requisitos
- Python 3.11+
- Windows 10/11

### Dependências
```bash
pip install pygame-ce Pillow pystray
```

### Como usar
```bash
# Clone o repositório
git clone https://github.com/isaque-rodriguess/deskBuddy.git
cd deskBuddy

# Execute
python pet.py
```

Ou duplo-clique em `iniciar.bat`.

---

## 🎮 Controles

| Ação | Como |
|------|------|
| **Arrastar pet** | Clique e arraste |
| **Menu** | Botão direito no pet |
| **Configurações** | Tray icon → Settings |
| **Mostrar/Esconder** | Tray icon → Show/Hide |
| **Sair** | Tray icon → Quit |

---

## 🖼️ Preview

O pet aparece como uma janela transparente sobre sua tela, andando pela barra de tarefas com animações de idle, running, jumping e waving.

---

## 📁 Estrutura

```
deskBuddy/
├── pet.py              # Código principal
├── pet.pyw             # Versão sem console
├── petdex_api.py       # API do Petdex
├── sprite_parser.py    # Parser de sprites
├── iniciar.bat         # Atalho de inicialização
├── make_shortcut.py    # Cria atalho na área de trabalho
└── .gitignore
```

---

## 🛠️ Tecnologias

- **Python 3.11** — linguagem principal
- **Tkinter** — interface gráfica e janela transparente
- **Pillow** — manipulação de imagens e sprites
- **Pystray** — ícone na bandeja do Windows
- **Petdex API** — catálogo de 3300+ pets

---

## 📄 Licença

MIT License — use como quiser!
