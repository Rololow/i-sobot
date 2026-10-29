/* Fonctions ROM de l'ESP32 utilisées par le blob Bluetooth d'esp-wifi-sys-esp32 0.3.0
 * mais absentes du script d'esp-rom-sys 0.1.5. Adresses reprises d'ESP-IDF :
 * components/esp_rom/esp32/ld/esp32.rom.ld */
PROVIDE ( ld_pscan_evt_start_cbk = 0x4003e924 );
PROVIDE ( ld_iscan_evt_start_cbk = 0x4003b58c );
PROVIDE ( ld_page_evt_start_cbk = 0x4003cf40 );
