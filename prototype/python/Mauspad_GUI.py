import tkinter as tk
from tkinter import colorchooser, ttk, messagebox
import hid
import time

VID = 0x04D9
PID = 0xA1FA

class MousepadGUI:
    def __init__(self, root):
        self.root = root
        self.root.title("Holtek RGB Controller (G-aim Alternative)")
        self.root.geometry("350x550")
        self.root.resizable(False, False)
        
        # Standard-Farben
        self.colors = [
            (255, 0, 0),     # Slot 1
            (0, 255, 0),     # Slot 2
            (0, 0, 255),     # Slot 3
            (255, 255, 0),   # Slot 4
            (0, 255, 255),   # Slot 5
            (255, 0, 255),   # Slot 6
            (255, 255, 255)  # Slot 7
        ]
        
        self.color_buttons = []
        
        # --- FARBEN BEREICH ---
        color_frame = tk.LabelFrame(root, text="Farben (Slots 1-7)", padx=10, pady=10)
        color_frame.pack(fill="x", padx=10, pady=5)
        
        for i in range(7):
            btn = tk.Button(color_frame, text=f"Slot {i+1} Farbe", bg=self.rgb_to_hex(self.colors[i]),
                            command=lambda idx=i: self.pick_color(idx), width=20)
            btn.pack(pady=2)
            # Textfarbe an Hintergrund anpassen
            self.update_btn_text_color(btn, self.colors[i])
            self.color_buttons.append(btn)
            
        # --- EFFEKTE BEREICH ---
        settings_frame = tk.LabelFrame(root, text="Effekt-Einstellungen", padx=10, pady=10)
        settings_frame.pack(fill="x", padx=10, pady=5)
        
        # Modus
        tk.Label(settings_frame, text="Modus:").grid(row=0, column=0, sticky="w", pady=5)
        modes = [("Statisch", "0x00"), ("Atmen (Breathe)", "0x01"), ("Neon", "0x02"), 
                 ("Regenbogen", "0x03"), ("Aus (Off)", "0x04")]
        self.mode_var = tk.StringVar()
        self.mode_cb = ttk.Combobox(settings_frame, textvariable=self.mode_var, 
                                    values=[m[0] for m in modes], state="readonly", width=15)
        self.mode_cb.current(0)
        self.mode_cb.grid(row=0, column=1, pady=5, padx=10)
        self.mode_map = {m[0]: int(m[1], 16) for m in modes}
        
        # Speed
        tk.Label(settings_frame, text="Speed (0-5):").grid(row=1, column=0, sticky="w")
        self.speed_var = tk.IntVar(value=4)
        tk.Scale(settings_frame, from_=0, to=5, orient="horizontal", variable=self.speed_var).grid(row=1, column=1, padx=10)
        
        # Brightness
        tk.Label(settings_frame, text="Helligkeit (0-32):").grid(row=2, column=0, sticky="w")
        self.bright_var = tk.IntVar(value=32)
        tk.Scale(settings_frame, from_=0, to=32, resolution=8, orient="horizontal", variable=self.bright_var).grid(row=2, column=1, padx=10)
        
        # --- APPLY BUTTON ---
        apply_btn = tk.Button(root, text="Zum Mauspad senden!", command=self.apply_to_device, 
                              bg="#2b2b2b", fg="white", font=("Arial", 11, "bold"))
        apply_btn.pack(pady=15, ipadx=10, ipady=5)
        
    def rgb_to_hex(self, rgb):
        return "#%02x%02x%02x" % rgb
        
    def update_btn_text_color(self, btn, rgb):
        # Wenn die Farbe dunkel ist, setze den Text auf Weiß, sonst Schwarz
        luminance = (0.299*rgb[0] + 0.587*rgb[1] + 0.114*rgb[2])
        btn.config(fg="white" if luminance < 128 else "black")
        
    def pick_color(self, idx):
        color = colorchooser.askcolor(initialcolor=self.rgb_to_hex(self.colors[idx]), title=f"Farbe für Slot {idx+1} wählen")
        if color[0]:
            rgb = (int(color[0][0]), int(color[0][1]), int(color[0][2]))
            self.colors[idx] = rgb
            self.color_buttons[idx].config(bg=color[1])
            self.update_btn_text_color(self.color_buttons[idx], rgb)
            
    def apply_to_device(self):
        mode = self.mode_map[self.mode_cb.get()]
        speed = self.speed_var.get()
        brightness = self.bright_var.get()
        
        found = False
        # Finde alle Interfaces des Mauspads
        for d in hid.enumerate(VID, PID):
            try:
                device = hid.device()
                device.open_path(d['path'])
                device.set_nonblocking(1)
                
                # 1. Hintergrundfarben hochladen
                packet1 = [0x00, 0x09, self.colors[1][0], self.colors[1][1], self.colors[1][2], 
                                     self.colors[2][0], self.colors[2][1], self.colors[2][2], 0x00]
                packet2 = [0x00, 0x0a, self.colors[3][0], self.colors[3][1], self.colors[3][2], 
                                     self.colors[4][0], self.colors[4][1], self.colors[4][2], 0x00]
                packet3 = [0x00, 0x0b, self.colors[5][0], self.colors[5][1], self.colors[5][2], 
                                     self.colors[6][0], self.colors[6][1], self.colors[6][2], 0x00]
                
                device.send_feature_report(packet1)
                time.sleep(0.01)
                device.send_feature_report(packet2)
                time.sleep(0.01)
                device.send_feature_report(packet3)
                time.sleep(0.01)
                
                # 2. Statische Farben im internen Speicher aktualisieren
                for i in range(7):
                    slot_hex = i * 0x10
                    packet4 = [0x00, 0x08, 0x00, brightness, speed, slot_hex, self.colors[i][0], self.colors[i][1], self.colors[i][2]]
                    device.send_feature_report(packet4)
                    time.sleep(0.01)
                    
                # 3. Gewünschten Effekt ausführen
                final_packet = [0x00, 0x08, mode, brightness, speed, 0x00, self.colors[0][0], self.colors[0][1], self.colors[0][2]]
                device.send_feature_report(final_packet)
                time.sleep(0.05)
                
                device.close()
                found = True
            except Exception as e:
                pass
                
        if found:
            messagebox.showinfo("Erfolg", "Alle Einstellungen wurden auf das Mauspad übertragen!")
        else:
            messagebox.showerror("Fehler", "Mauspad nicht gefunden! Ist es angesteckt?")

if __name__ == "__main__":
    root = tk.Tk()
    app = MousepadGUI(root)
    root.mainloop()
