using Unity.Collections;

namespace KimoTech.LichoraHost
{
    public interface IBrowserFrameSource
    {
        bool IsReady { get; }
        int FrameWidth { get; }
        int FrameHeight { get; }

        bool TryCopyFrameTo(NativeArray<byte> destination);
    }
}
