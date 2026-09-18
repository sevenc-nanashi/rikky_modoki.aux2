-- https://canary.discord.com/channels/1392018499072823327/1417446601642741803/1417446601642741803

--information:Unsafeスクリプト制御 r3
--label:Unsafe
--filter
--text@code:コード,

if #code > 0 then assert(loadstring(code))() end
