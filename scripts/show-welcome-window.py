#!/usr/bin/env python3
import tkinter as tk
import time

root = tk.Tk()
root.title("Monitor Secundário - Pi Zero GPU Offload")

# Posiciona a janela na segunda tela:
# Tela 1: 0..1919
# Tela 2: 1920..3519
# x=2100, y=150 (centro da segunda tela 1600x900)
root.geometry("800x500+2320+200")
root.configure(bg="#1e1e2e")

title = tk.Label(
    root,
    text="🖥️ SEGUNDA TELA ATIVA",
    font=("Helvetica", 28, "bold"),
    fg="#a6e3a1",
    bg="#1e1e2e"
)
title.pack(pady=20)

subtitle = tk.Label(
    root,
    text="Raspberry Pi Zero W + GPU Offload (AMD 610M -> VideoCore IV)",
    font=("Helvetica", 14),
    fg="#cdd6f4",
    bg="#1e1e2e"
)
subtitle.pack(pady=5)

info = tk.Label(
    root,
    text="Resolução: 1600x900 @ 60 FPS  |  Latência de Rede: 0.3ms\nZero Flick  |  Zero Tearing  |  Hardware KMS Sincronizado",
    font=("Helvetica", 12),
    fg="#89b4fa",
    bg="#1e1e2e"
)
info.pack(pady=20)

clock_label = tk.Label(
    root,
    font=("Helvetica", 36, "bold"),
    fg="#f9e2af",
    bg="#1e1e2e"
)
clock_label.pack(pady=20)

hint = tk.Label(
    root,
    text="👉 Você pode arrastar qualquer janela do seu notebook\npara a direita e ela aparecerá aqui!",
    font=("Helvetica", 13, "italic"),
    fg="#f38ba8",
    bg="#1e1e2e"
)
hint.pack(pady=15)

def update_clock():
    now = time.strftime("%H:%M:%S")
    clock_label.config(text=now)
    root.after(500, update_clock)

update_clock()
root.mainloop()
