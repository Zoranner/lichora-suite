using UnityEngine;

namespace KimoTech.LichoraHost
{
    /// <summary>
    /// MouseState 使用 struct 以减少 GC 压力（小对象值类型更高效）
    /// </summary>
    public struct MouseState
    {
        public bool Effective;
        public Vector2 Position;
        public Vector2 Delta;
        public bool LeftButton;
        public bool RightButton;
        public bool MiddleButton;
        public Vector2 ScrollDelta;
    }

    [DisallowMultipleComponent]
    [RequireComponent(typeof(RectTransform))]
    public class PointableUI : BrowserInputController { }
}
