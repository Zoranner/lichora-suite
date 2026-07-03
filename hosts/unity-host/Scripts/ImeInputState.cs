using System;
using System.Linq;
using UnityEngine;

namespace KimoTech.LichoraHost
{
    public sealed class ImeInputState
    {
        private readonly Action<string, int, int> _SetComposition;
        private readonly Action<string> _CommitText;
        private readonly Action _CancelComposition;
        private readonly Action<int, int> _DeleteSurroundingText;
        private string _LastCompositionString = "";
        private bool _WasComposing;
        private bool _JustCommitted;

        public ImeInputState(
            Action<string, int, int> setComposition,
            Action<string> commitText,
            Action cancelComposition,
            Action<int, int> deleteSurroundingText
        )
        {
            _SetComposition =
                setComposition ?? throw new ArgumentNullException(nameof(setComposition));
            _CommitText = commitText ?? throw new ArgumentNullException(nameof(commitText));
            _CancelComposition =
                cancelComposition ?? throw new ArgumentNullException(nameof(cancelComposition));
            _DeleteSurroundingText =
                deleteSurroundingText
                ?? throw new ArgumentNullException(nameof(deleteSurroundingText));
        }

        public bool UpdateComposition(string compositionString, string inputString)
        {
            var isComposing = !string.IsNullOrEmpty(compositionString);
            var hasCommitText = !string.IsNullOrEmpty(inputString);
            var hasImeInput = false;

            if (_JustCommitted)
            {
                if (!hasCommitText)
                {
                    _JustCommitted = false;
                }
                else
                {
                    return true;
                }
            }

            var hasNonAsciiChar = hasCommitText && inputString.Any(c => c >= 128);
            if (hasCommitText && (_WasComposing || hasNonAsciiChar))
            {
                CommitText(inputString);
                _LastCompositionString = "";
                _WasComposing = false;
                _JustCommitted = true;
                hasImeInput = true;
            }
            else if (isComposing)
            {
                if (compositionString != _LastCompositionString)
                {
                    SetComposition(
                        compositionString,
                        compositionString.Length,
                        compositionString.Length
                    );
                    _LastCompositionString = compositionString;
                }

                _WasComposing = true;
                hasImeInput = true;
            }
            else if (_WasComposing)
            {
                CancelComposition();
                _LastCompositionString = "";
                _WasComposing = false;
                hasImeInput = true;
            }

            return hasImeInput;
        }

        public void ResetState()
        {
            if (_WasComposing)
            {
                CancelComposition();
            }

            _LastCompositionString = "";
            _WasComposing = false;
            _JustCommitted = false;
        }

        public void SetComposition(string text, int selectionStart, int selectionEnd)
        {
            if (string.IsNullOrEmpty(text))
            {
                return;
            }

            _SetComposition(text, selectionStart, selectionEnd);
        }

        public void CommitText(string text)
        {
            if (string.IsNullOrEmpty(text))
            {
                return;
            }

            _CommitText(text);
        }

        public void CancelComposition()
        {
            _CancelComposition();
        }

        public void DeleteSurroundingText(int before, int after)
        {
            if (before <= 0 && after <= 0)
            {
                return;
            }

            if (before > short.MaxValue || after > short.MaxValue)
            {
                Debug.LogWarning(
                    $"IME DeleteSurroundingText range too large: before={before}, after={after}"
                );
                return;
            }

            _DeleteSurroundingText(before, after);
        }
    }
}
