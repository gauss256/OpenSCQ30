package com.oppzippy.openscq30.features.soundcoredevice.service

import android.content.Context
import android.media.AudioManager
import android.util.Log
import android.view.KeyEvent
import com.oppzippy.openscq30.lib.bindings.OpenScq30Device
import com.oppzippy.openscq30.lib.wrapper.Setting
import kotlin.time.Duration
import kotlin.time.Duration.Companion.milliseconds
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/**
 * Pauses media playback when the earbuds report that an Easy Chat session has started, and
 * resumes it when the session ends.
 *
 * Easy Chat (Liberty 5 Pro and similar) is implemented in the earbud firmware: it ducks the audio
 * locally and switches to transparency mode, but the phone keeps streaming, so podcasts and
 * audiobooks keep running silently during the conversation. The earbuds announce the start and
 * end of the session over the Soundcore data channel, which the Rust library exposes as the
 * read-only `easyChatActive` setting.
 *
 * The buds send the "ended" event when voice detection closes, but they keep ducking and
 * transparency active for a short tail (about four seconds) before restoring the previous sound
 * mode. Resuming immediately would play into that tail, so resume is delayed by [resumeDelay] and
 * canceled if a new session starts first.
 */
class EasyChatMediaController(
    context: Context,
    private val scope: CoroutineScope,
    private val resumeDelay: Duration = DEFAULT_RESUME_DELAY,
) {
    companion object {
        private const val TAG = "EasyChatMediaController"
        const val SETTING_ID = "easyChatActive"
        private const val ACTIVE_VALUE = "Active"

        /** Roughly the observed gap between the end event and the earbuds restoring the mode. */
        val DEFAULT_RESUME_DELAY = 4000.milliseconds
    }

    private val audioManager: AudioManager? = context.getSystemService(AudioManager::class.java)
    private var wasActive = false
    private var pausedByUs = false
    private var pendingResume: Job? = null

    /** Call whenever the device state may have changed. Safe to call frequently. */
    fun onDeviceStateChanged(device: OpenScq30Device) {
        val setting = try {
            device.setting(SETTING_ID)
        } catch (ex: IllegalStateException) {
            Log.w(TAG, "device was closed, ignoring state change", ex)
            return
        }
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
            // A new session cancels any resume still waiting out the previous session's tail.
            pendingResume?.cancel()
            pendingResume = null
            if (audioManager.isMusicActive) {
                Log.i(TAG, "easy chat started, pausing playback")
                dispatchMediaKey(audioManager, KeyEvent.KEYCODE_MEDIA_PAUSE)
                pausedByUs = true
            } else {
                Log.i(TAG, "easy chat started, nothing playing")
            }
        } else if (pausedByUs) {
            Log.i(TAG, "easy chat ended, resuming playback in $resumeDelay")
            pendingResume?.cancel()
            pendingResume = scope.launch {
                delay(resumeDelay)
                Log.i(TAG, "resuming playback")
                dispatchMediaKey(audioManager, KeyEvent.KEYCODE_MEDIA_PLAY)
                pausedByUs = false
                pendingResume = null
            }
        } else {
            Log.i(TAG, "easy chat ended, nothing to resume")
        }
    }

    private fun dispatchMediaKey(audioManager: AudioManager, keyCode: Int) {
        audioManager.dispatchMediaKeyEvent(KeyEvent(KeyEvent.ACTION_DOWN, keyCode))
        audioManager.dispatchMediaKeyEvent(KeyEvent(KeyEvent.ACTION_UP, keyCode))
    }
}
