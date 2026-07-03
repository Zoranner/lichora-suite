// ============================================================
// Project: NativeProcessSpawner
// Description: P/Invoke wrapper for the native process host plugin.
//              Works on both Windows (process_host.dll) and
//              Linux (libprocess_host.so).
// ============================================================

using System;
using System.Runtime.InteropServices;
using System.Threading;
using UnityEngine;

namespace KimoTech.EmbeddedBrowser
{
    public static class NativeProcessSpawner
    {
        private const string PLUGIN = "process_host";

        [DllImport(
            PLUGIN,
            EntryPoint = "process_host_spawn",
            CallingConvention = CallingConvention.Cdecl,
            CharSet = CharSet.Ansi
        )]
        private static extern int ProcessHostSpawn(string executable, string args);

        [DllImport(
            PLUGIN,
            EntryPoint = "process_host_wait",
            CallingConvention = CallingConvention.Cdecl
        )]
        private static extern int ProcessHostWait(int pid);

        [DllImport(
            PLUGIN,
            EntryPoint = "process_host_is_running",
            CallingConvention = CallingConvention.Cdecl
        )]
        private static extern int ProcessHostIsRunning(int pid);

        [DllImport(
            PLUGIN,
            EntryPoint = "process_host_kill",
            CallingConvention = CallingConvention.Cdecl
        )]
        private static extern int ProcessHostKill(int pid, int force);

        // ----------------------------------------------------------------

        /// <summary>
        /// Spawn <paramref name="executable"/> with <paramref name="args"/> as
        /// a single argument. Returns the child PID, or -1 on failure.
        /// </summary>
        public static int Spawn(string executable, string args)
        {
            try
            {
                var pid = ProcessHostSpawn(executable, args);
                if (pid > 0)
                {
                    Debug.Log($"[NativeProcessSpawner] Spawned '{executable} {args}' (PID={pid})");
                }
                else
                {
                    Debug.LogError($"[NativeProcessSpawner] Failed to spawn '{executable} {args}'");
                }

                return pid;
            }
            catch (DllNotFoundException)
            {
                Debug.LogError(
                    "[NativeProcessSpawner] Plugin not found. "
                        + "Build process-host and place it in "
                        + "hosts/unity-host/Plugins/<Platform>/"
                );
                return -1;
            }
        }

        /// <summary>
        /// Block until the process exits. Returns the exit code, or -1 on error.
        /// </summary>
        public static int Wait(int pid)
        {
            if (pid <= 0)
            {
                return -1;
            }

            try
            {
                return ProcessHostWait(pid);
            }
            catch (ThreadAbortException)
            {
                throw;
            }
            catch (Exception ex)
            {
                Debug.LogWarning($"[NativeProcessSpawner] Wait failed: {ex.Message}");
                return -1;
            }
        }

        /// <summary>
        /// Returns true if the process is still running.
        /// </summary>
        public static bool IsRunning(int pid)
        {
            if (pid <= 0)
            {
                return false;
            }

            try
            {
                return ProcessHostIsRunning(pid) == 1;
            }
            catch (Exception ex)
            {
                Debug.LogWarning($"[NativeProcessSpawner] IsRunning failed: {ex.Message}");
                return false;
            }
        }

        /// <summary>
        /// Terminate the process.
        /// <paramref name="force"/> = false: SIGTERM (Unix) / TerminateProcess (Windows).
        /// <paramref name="force"/> = true:  SIGKILL (Unix) / TerminateProcess (Windows).
        /// Returns 0 on success, -1 on error.
        /// </summary>
        public static int Kill(int pid, bool force = false)
        {
            if (pid <= 0)
            {
                return -1;
            }

            try
            {
                return ProcessHostKill(pid, force ? 1 : 0);
            }
            catch (Exception ex)
            {
                Debug.LogWarning($"[NativeProcessSpawner] Kill failed: {ex.Message}");
                return -1;
            }
        }
    }
}
