using System.Threading;
using Cysharp.Threading.Tasks;
using KimoTech.LichoraHost;
using UnityEngine;

public class ScriptSender : MonoBehaviour
{
    public PageRenderer PageRenderer;

    public int DataCount;

    private CancellationTokenSource _CancellationTokenSource;

    private void Start()
    {
        _CancellationTokenSource = new CancellationTokenSource();
        CopyTextureTask(_CancellationTokenSource.Token).Forget();
    }

    private async UniTaskVoid CopyTextureTask(CancellationToken token)
    {
        try
        {
            await UniTask.Delay(3000, cancellationToken: token);

            while (!token.IsCancellationRequested)
            {
                if (PageRenderer != null)
                {
                    DataCount++;
                    PageRenderer.ExecuteScript("addRandomData();");
                }

                await UniTask.Delay(100, cancellationToken: token);
            }
        }
        catch (System.OperationCanceledException)
        {
            // 正常取消，不需要记录
        }
    }

    private void OnDestroy()
    {
        _CancellationTokenSource?.Cancel();
        _CancellationTokenSource?.Dispose();
    }
}
