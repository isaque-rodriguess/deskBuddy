# Desktop Pet Zero

Um pet de desktop animado que anda pela sua tela com animacoes fluidas. Escolha entre **3300+ pets** do [Petdex](https://petdex.dev) ou use os pets instalados do Hermes.

![Python](https://img.shields.io/badge/Python-3.11+-blue?logo=python&logoColor=white)
![Platform](https://img.shields.io/badge/Platform-Windows-0078d4?logo=windows&logoColor=white)

---

## Funcionalidades

- **3300+ pets** -- baixe qualquer pet do Petdex direto pelo menu
- **Roam** -- o pet anda pela tela automaticamente com pausas e animacoes
- **Gravidade** -- solte o pet no ar e ele cai com animacao de jump
- **Arrastavel** -- clique e arraste o pet pra qualquer lugar da tela
- **Busca** -- encontre pets por nome ou slug
- **Configuracoes** -- tamanho, ativar/desativar, roaming
- **Dark theme** -- interface moderna e escura
- **Tray icon** -- minimize pra bandeja do Windows

---

## Instalacao

### Pre-requisitos
- Python 3.11+
- Windows 10/11

### Dependencias
```bash
pip install Pillow pystray
```

### Como usar
```bash
git clone https://github.com/isaque-rodriguess/deskBuddy.git
cd deskBuddy
python pet.py
```

Ou duplo-clique em `iniciar.bat`.

---

## Controles

| Acao | Como |
|------|------|
| **Arrastar pet** | Clique e arraste |
| **Menu** | Botao direito no pet |
| **Configuracoes** | Tray icon - Settings |
| **Mostrar/Esconder** | Tray icon - Show/Hide |
| **Sair** | Tray icon - Quit |

---

## Estrutura

```
deskBuddy/
  pet.py              # Codigo principal
  pet.pyw             # Versao sem console
  petdex_api.py       # API do Petdex
  sprite_parser.py    # Parser de sprites
  iniciar.bat         # Atalho de inicializacao
  make_shortcut.py    # Cria atalho na area de trabalho
```

---

## Tecnologias

- **Python 3.11** -- linguagem principal
- **Tkinter** -- interface grafica e janela transparente
- **Pillow** -- manipulacao de imagens e sprites
- **Pystray** -- icone na bandeja do Windows
- **Petdex API** -- catalogo de 3300+ pets
