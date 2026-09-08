package com.oppzippy.openscq30.features.soundcoredevice.service

import android.content.Context
import android.media.AudioManager
import android.util.Log
import android.view.KeyEvent
import com.oppzippy.openscq30.lib.bindings.OpenScq30Device
import com.oppzippy.openscq30.lib.wrapper.Setting

/**
 * Pauses media playback when the earbuds report that an Easy Chat session has started, and
 * resumes it when the session ends.
 *
 * Easy Chat (Liberty 5 Pro and similar) is implemented in the earbud firmware: it ducks the audio
 * locally and switches to transparency mode, but the phone keeps streaming, so podcasts and
 * audiobooks keep running silently during the conversation. The earbuds do announce the start and
 * end of the session over the Soundcore data channel, which the Rust library exposes as the
 * read-only `easyChatActive` setting. This class turns those transitions into media key events.
 */
class EasyChatMediaController(context: Context) {
    companion object {
        private const val TAG = "EasyChatMediaController"
        const val SETTING_ID = "easyChatActive"
        private const val ACTIVE_VALUE = "Active"
    }

    private val audioManager: AudioManager? = context.getSystemService(AudioManager::class.java)
    private var wasActive = false
    private var pausedByUs = false

    /** Call whenever the device state may have changed. Safe to call frequently. */
    fun onDeviceStateChanged(device: OpenScq30Device) {
        val setting = try {
            device.setting(SETTING_ID)
        } catch (ex: IllegalStateException) {
            Log.w(TAG, "device was closed, ignoring state change", ex)
            return
        }
        Log.d(TAG, "state changed, $SETTING_ID = $setting")
        // Devices without the setting are simply ignored
        if (setting !is Setting.InformationSetting) return
        onEasyChatActive(setting.value == ACTIVE_VALUE)
    }

    fun onEasyChatActive(isActive: Boolean) {
        if (isActive == wasActive) return
        wasActive = isActive
        val audioManager = audioManager ?: run {
            Log.e(TAG, "AudioManager unavailable, can't control playback")
            return
        }
        if (isActive) {
            if (audioManager.isMusicActive) {
                Log.i(TAG, "easy chat started, pausing playback")
                dispatchMediaKey(audioManager, KeyEvent.KEYCODE_MEDIA_PAUSE)
                pausedByUs = true
            } else {
                Log.i(TAG, "easy chat started, nothing playing")
            }
        } else if (pausedByUs) {
            Log.i(TAG, "easy chat ended, resuming playback")
            dispatchMediaKey(audioManager, KeyEvent.KEYCODE_MEDIA_PLAY)
            pausedByUs = false
        } else {
            Log.i(TAG, "easy chat ended, nothing to resume")
        }
    }

    private fun dispatchMediaKey(audioManager: AudioManager, keyCode: Int) {
        audioManager.dispatchMediaKeyEvent(KeyEvent(KeyEvent.ACTION_DOWN, keyCode))
        audioManager.dispatchMediaKeyEvent(KeyEvent(KeyEvent.ACTION_UP, keyCode))
    }
}
