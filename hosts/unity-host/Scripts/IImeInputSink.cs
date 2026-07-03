namespace KimoTech.LichoraHost
{
    public interface IImeInputSink
    {
        void CommitImeText(string text);
        void SetImeComposition(string text, int selectionStart, int selectionEnd);
        void CancelImeComposition();
        void DeleteImeSurroundingText(int before, int after);
    }
}
