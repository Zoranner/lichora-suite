using UnityEngine;
using UnityEngine.EventSystems;

namespace KimoTech.LichoraHost
{
    public sealed class BrowserPointerInputSource
    {
        private readonly BrowserCoordinateMapper _CoordinateMapper;
        private MouseState _State;

        public BrowserPointerInputSource(BrowserCoordinateMapper coordinateMapper)
        {
            _CoordinateMapper = coordinateMapper;
        }

        public MouseState State => _State;

        public void Enter()
        {
            _State.Effective = true;
        }

        public void Exit()
        {
            _State.Effective = false;
        }

        public void Move(PointerEventData eventData)
        {
            _State.Effective = true;
            UpdatePosition(eventData);
        }

        public void Press(PointerEventData eventData)
        {
            UpdatePosition(eventData);
            SetButton(eventData.button, true);
        }

        public void Release(PointerEventData eventData)
        {
            UpdatePosition(eventData);
            SetButton(eventData.button, false);
        }

        public void Scroll(PointerEventData eventData)
        {
            UpdatePosition(eventData);
            _State.ScrollDelta += eventData.scrollDelta;
        }

        public void ResetFrameDeltas()
        {
            _State.ScrollDelta = Vector2.zero;
            _State.Delta = Vector2.zero;
        }

        private void SetButton(PointerEventData.InputButton button, bool pressed)
        {
            switch (button)
            {
                case PointerEventData.InputButton.Left:
                    _State.LeftButton = pressed;
                    break;
                case PointerEventData.InputButton.Right:
                    _State.RightButton = pressed;
                    break;
                case PointerEventData.InputButton.Middle:
                    _State.MiddleButton = pressed;
                    break;
            }
        }

        private void UpdatePosition(PointerEventData eventData)
        {
            var position = _CoordinateMapper.ScreenPointToBrowserNormalized(
                eventData.position,
                eventData.pressEventCamera != null
                    ? eventData.pressEventCamera
                    : eventData.enterEventCamera
            );
            _State.Delta += position - _State.Position;
            _State.Position = position;
        }
    }
}
