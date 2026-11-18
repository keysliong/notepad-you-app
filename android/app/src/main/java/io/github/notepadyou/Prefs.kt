package io.github.notepadyou

import android.content.Context
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf

/** Настройки приложения (SharedPreferences), наблюдаемые из Compose. */
class Prefs(context: Context) {
    private val sp = context.getSharedPreferences("settings", Context.MODE_PRIVATE)

    private val _dynamic = mutableStateOf(sp.getBoolean("dynamic", false))
    var dynamic: Boolean
        get() = _dynamic.value
        set(v) { _dynamic.value = v; sp.edit().putBoolean("dynamic", v).apply() }

    private val _seed = mutableStateOf(runCatching { Seed.valueOf(sp.getString("seed", "Lavender")!!) }.getOrDefault(Seed.Lavender))
    var seed: Seed
        get() = _seed.value
        set(v) { _seed.value = v; sp.edit().putString("seed", v.name).apply() }

    /** 0 — как в системе, 1 — светлая, 2 — тёмная */
    private val _themeMode = mutableIntStateOf(sp.getInt("theme", 0))
    var themeMode: Int
        get() = _themeMode.intValue
        set(v) { _themeMode.intValue = v; sp.edit().putInt("theme", v).apply() }

    private val _fontSize = mutableFloatStateOf(sp.getFloat("fontSize", 17f))
    var fontSize: Float
        get() = _fontSize.floatValue
        set(v) { _fontSize.floatValue = v; sp.edit().putFloat("fontSize", v).apply() }

    private val _nunito = mutableStateOf(sp.getBoolean("nunito", true))
    var nunito: Boolean
        get() = _nunito.value
        set(v) { _nunito.value = v; sp.edit().putBoolean("nunito", v).apply() }

    private val _onboarded = mutableStateOf(sp.getBoolean("onboarded", false))
    var onboarded: Boolean
        get() = _onboarded.value
        set(v) { _onboarded.value = v; sp.edit().putBoolean("onboarded", v).apply() }

    var seeded: Boolean
        get() = sp.getBoolean("seeded", false)
        set(v) { sp.edit().putBoolean("seeded", v).apply() }
}
