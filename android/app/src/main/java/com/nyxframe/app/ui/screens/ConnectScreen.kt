package com.nyxframe.app.ui.screens

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.*
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.Image
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.layout.ContentScale
import com.nyxframe.app.R
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.Shadow
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.withTransform
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.foundation.Canvas
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.nyxframe.app.ui.viewmodel.AgentViewModel
import com.nyxframe.app.ui.viewmodel.DiscoveredWorkstation
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.ui.platform.LocalContext

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ConnectScreen(
    viewModel: AgentViewModel,
    onNavigateToStream: () -> Unit,
    onNavigateToSettings: () -> Unit
) {
    var ipInput by remember { mutableStateOf(viewModel.serverHost) }

    val context = LocalContext.current



    val uploadFilesLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.OpenMultipleDocuments()
    ) { uris ->
        if (uris.isNotEmpty()) {
            viewModel.pendingUploadUris = uris
            viewModel.isPendingUploadDirectory = false
            viewModel.isFileSharingActive = true
            viewModel.fileSharingMode = 0
            viewModel.fetchRemoteDirectory(null)
        }
    }

    val uploadFolderLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.OpenDocumentTree()
    ) { uri ->
        if (uri != null) {
            viewModel.pendingUploadUris = listOf(uri)
            viewModel.isPendingUploadDirectory = true
            viewModel.isFileSharingActive = true
            viewModel.fileSharingMode = 0
            viewModel.fetchRemoteDirectory(null)
        }
    }

    val downloadFolderLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.OpenDocumentTree()
    ) { uri ->
        if (uri != null) {
            viewModel.downloadSelectedHostItems(
                targetHostPaths = viewModel.remoteSelectedItems.toList(),
                localFolderUri = uri,
                onSuccess = {
                    android.widget.Toast.makeText(context, "DOWNLOAD SUCCESSFUL", android.widget.Toast.LENGTH_SHORT).show()
                },
                onError = { err ->
                    android.widget.Toast.makeText(context, "DOWNLOAD FAILED: $err", android.widget.Toast.LENGTH_LONG).show()
                }
            )
        }
    }

    var showNewFolderDialog by remember { mutableStateOf(false) }
    var newFolderName by remember { mutableStateOf("") }

    val accentCyan = viewModel.themePrimary
    val accentPurple = viewModel.themeSecondary
    val panelBg = viewModel.themePanel

    val accentGradient = Brush.horizontalGradient(
        colors = listOf(accentCyan, accentPurple)
    )

    val darkGradient = Brush.verticalGradient(
        colors = listOf(
            viewModel.themeBackground,
            Color(
                red = (viewModel.themeBackground.red * 0.4f),
                green = (viewModel.themeBackground.green * 0.4f),
                blue = (viewModel.themeBackground.blue * 0.4f),
                alpha = viewModel.themeBackground.alpha
            )
        )
    )

    val borderGradient = Brush.verticalGradient(
        colors = listOf(accentCyan.copy(alpha = 0.35f), Color.Transparent)
    )

    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(darkGradient)
    ) {
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(horizontal = 24.dp, vertical = 32.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(20.dp)
        ) {
            // Brand Header Section
            item {
                Spacer(modifier = Modifier.height(20.dp))
                // Premium NyxFrame Brand Icon
                Box(
                    modifier = Modifier
                        .size(80.dp)
                        .clip(RoundedCornerShape(topStart = 16.dp, bottomEnd = 16.dp))
                        .background(Color(0xFF0E1624)) // NYX_SURFACE
                        .border(1.dp, accentCyan.copy(alpha = 0.5f), RoundedCornerShape(topStart = 16.dp, bottomEnd = 16.dp)),
                    contentAlignment = Alignment.Center
                ) {
                    // Internal soft static cyan glow
                    Box(
                        modifier = Modifier
                            .fillMaxSize()
                            .background(
                                Brush.radialGradient(
                                    colors = listOf(
                                        accentCyan.copy(alpha = 0.2f),
                                        Color.Transparent
                                    )
                                )
                            )
                    )

                    // NyxFrame Vector Logo
                    Image(
                        painter = painterResource(id = R.mipmap.ic_launcher_foreground),
                        contentDescription = "NyxFrame Logo",
                        modifier = Modifier.size(90.dp),
                        contentScale = ContentScale.Fit
                    )
                }

                Spacer(modifier = Modifier.height(20.dp))

                Text(
                    text = "NYXFRAME",
                    style = TextStyle(
                        color = Color.White,
                        fontSize = 26.sp,
                        fontWeight = FontWeight.Black,
                        letterSpacing = 4.sp,
                        shadow = Shadow(
                            color = accentCyan.copy(alpha = 0.7f),
                            offset = Offset(0f, 0f),
                            blurRadius = 14f
                        )
                    ),
                    textAlign = TextAlign.Center
                )

                Box(
                    modifier = Modifier
                        .padding(top = 4.dp)
                        .height(2.dp)
                        .width(40.dp)
                        .background(accentCyan)
                )

                Text(
                    text = "Remote Workstation Console",
                    color = Color(0xFF94A3B8), // NYX_TEXT_2
                    fontSize = 13.sp,
                    fontWeight = FontWeight.Bold,
                    textAlign = TextAlign.Center,
                    modifier = Modifier.padding(top = 8.dp)
                )

                Spacer(modifier = Modifier.height(20.dp))
            }

            // Connection Panel (Manual Input Card)
            item {
                Card(
                    shape = RoundedCornerShape(topStart = 16.dp, bottomEnd = 16.dp),
                    colors = CardDefaults.cardColors(containerColor = panelBg),
                    modifier = Modifier
                        .fillMaxWidth()
                        .border(1.dp, borderGradient, RoundedCornerShape(topStart = 16.dp, bottomEnd = 16.dp))
                ) {
                    Column(
                        modifier = Modifier.padding(24.dp)
                    ) {
                        Text(
                            text = "WORKSTATION IP ADDRESS OR HOSTNAME",
                            color = accentCyan,
                            fontSize = 11.sp,
                            fontWeight = FontWeight.ExtraBold,
                            letterSpacing = 1.sp
                        )

                        Spacer(modifier = Modifier.height(10.dp))

                        BasicTextField(
                            value = ipInput,
                            onValueChange = { ipInput = it },
                            textStyle = TextStyle(
                                color = Color.White,
                                fontSize = 14.sp,
                                fontWeight = FontWeight.Bold
                            ),
                            cursorBrush = SolidColor(accentCyan),
                            singleLine = true,
                            modifier = Modifier
                                .fillMaxWidth()
                                .height(48.dp)
                                .clip(RoundedCornerShape(14.dp))
                                .background(Color(0xFF0E1624))
                                .border(1.dp, accentCyan.copy(alpha = 0.3f), RoundedCornerShape(14.dp))
                                .padding(horizontal = 14.dp),
                            decorationBox = { innerTextField ->
                                Box(
                                    modifier = Modifier.fillMaxSize(),
                                    contentAlignment = Alignment.CenterStart
                                ) {
                                    if (ipInput.isEmpty()) {
                                        Text(
                                            text = "e.g. nyxframe or 100.86.174.101",
                                            color = Color(0xFF607086),
                                            fontSize = 14.sp,
                                            fontWeight = FontWeight.Bold
                                        )
                                    }
                                    innerTextField()
                                }
                            }
                        )

                        Spacer(modifier = Modifier.height(20.dp))

                        val isConnected = viewModel.isConnected
                        val buttonText = if (isConnected) "CONNECT" else "START LINK"
                        val buttonBg = if (isConnected) {
                            Brush.horizontalGradient(listOf(Color(0xFF0F9B4E), Color(0xFF004D20)))
                        } else {
                            accentGradient
                        }

                        Button(
                            onClick = {
                                if (isConnected) {
                                    onNavigateToStream()
                                } else {
                                    viewModel.connectToWorkstation(ipInput.trim())
                                }
                            },
                            colors = ButtonDefaults.buttonColors(containerColor = Color.Transparent),
                            contentPadding = PaddingValues(),
                            shape = RoundedCornerShape(topStart = 12.dp, bottomEnd = 12.dp),
                            modifier = Modifier
                                .fillMaxWidth()
                                .height(52.dp)
                                .background(buttonBg, RoundedCornerShape(topStart = 12.dp, bottomEnd = 12.dp))
                        ) {
                            if (viewModel.isConnecting) {
                                CircularProgressIndicator(
                                    color = Color.White,
                                    modifier = Modifier.size(24.dp)
                                )
                            } else {
                                Text(
                                    text = buttonText,
                                    color = Color.White,
                                    fontSize = 14.sp,
                                    fontWeight = FontWeight.Bold,
                                    letterSpacing = 2.sp
                                )
                            }
                        }

                        if (isConnected) {
                            Spacer(modifier = Modifier.height(10.dp))
                            Button(
                                onClick = {
                                    viewModel.disconnect()
                                },
                                colors = ButtonDefaults.buttonColors(containerColor = Color.Transparent),
                                contentPadding = PaddingValues(),
                                shape = RoundedCornerShape(topStart = 12.dp, bottomEnd = 12.dp),
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .height(48.dp)
                                    .background(
                                        Brush.horizontalGradient(listOf(Color(0xFF1A0E2B), Color(0xFF0A0714))),
                                        RoundedCornerShape(topStart = 12.dp, bottomEnd = 12.dp)
                                    )
                                    .border(1.dp, Color(0xFF00E5FF).copy(alpha = 0.4f), RoundedCornerShape(topStart = 12.dp, bottomEnd = 12.dp))
                            ) {
                                Text(
                                    text = "DISCONNECT LINK",
                                    color = Color.White,
                                    fontSize = 13.sp,
                                    fontWeight = FontWeight.Bold,
                                    letterSpacing = 1.5.sp
                                )
                            }
                        }
                    }
                }
            }

            // Symmetrical UI Action Buttons above the DISCOVERED WORKSTATIONS header
            item {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 4.dp),
                    horizontalArrangement = Arrangement.spacedBy(16.dp)
                ) {
                    var showUploadMenu by remember { mutableStateOf(false) }

                    // PUSH (UPLOAD) CAPSULAR BUTTON
                    Box(
                        modifier = Modifier
                            .weight(1f)
                            .height(44.dp)
                            .clip(RoundedCornerShape(100.dp))
                            .background(
                                Brush.horizontalGradient(
                                    colors = if (viewModel.isConnected) {
                                        listOf(accentCyan, accentPurple)
                                    } else {
                                        listOf(Color(0xFF151F31), Color(0xFF0E1624))
                                    }
                                )
                            )
                            .border(
                                1.dp,
                                if (viewModel.isConnected) accentCyan.copy(alpha = 0.6f) else Color(0xFF243247),
                                RoundedCornerShape(100.dp)
                            )
                            .clickable {
                                if (!viewModel.isConnected) {
                                    android.widget.Toast.makeText(context, "CONNECT WORKSTATION FOR FILE SHARING", android.widget.Toast.LENGTH_SHORT).show()
                                } else {
                                    showUploadMenu = true
                                }
                            },
                        contentAlignment = Alignment.Center
                    ) {
                        Text(
                            text = "PUSH (UPLOAD)",
                            color = if (viewModel.isConnected) Color.White else Color(0xFF607086),
                            fontSize = 12.sp,
                            fontWeight = FontWeight.ExtraBold,
                            letterSpacing = 1.5.sp
                        )

                        DropdownMenu(
                            expanded = showUploadMenu,
                            onDismissRequest = { showUploadMenu = false },
                            modifier = Modifier
                                .background(panelBg)
                                .border(1.dp, accentCyan.copy(alpha = 0.5f), RoundedCornerShape(8.dp))
                        ) {
                            DropdownMenuItem(
                                text = {
                                    Text(
                                        text = "UPLOAD FILES",
                                        color = Color.White,
                                        fontSize = 12.sp,
                                        fontWeight = FontWeight.Bold,
                                        letterSpacing = 1.sp
                                    )
                                },
                                onClick = {
                                    showUploadMenu = false
                                    uploadFilesLauncher.launch(arrayOf("*/*"))
                                }
                            )
                            Box(modifier = Modifier.fillMaxWidth().height(1.dp).background(accentCyan.copy(alpha = 0.2f)))
                            DropdownMenuItem(
                                text = {
                                    Text(
                                        text = "UPLOAD FOLDER",
                                        color = Color.White,
                                        fontSize = 12.sp,
                                        fontWeight = FontWeight.Bold,
                                        letterSpacing = 1.sp
                                    )
                                },
                                onClick = {
                                    showUploadMenu = false
                                    uploadFolderLauncher.launch(null)
                                }
                            )
                        }
                    }

                    // GET (DOWNLOAD) CAPSULAR BUTTON
                    Box(
                        modifier = Modifier
                            .weight(1f)
                            .height(44.dp)
                            .clip(RoundedCornerShape(100.dp))
                            .background(
                                Brush.horizontalGradient(
                                    colors = if (viewModel.isConnected) {
                                        listOf(Color(0xFF00D68F), Color(0xFF00593B))
                                    } else {
                                        listOf(Color(0xFF151F31), Color(0xFF0E1624))
                                    }
                                )
                            )
                            .border(
                                1.dp,
                                if (viewModel.isConnected) Color(0xFF00D68F).copy(alpha = 0.6f) else Color(0xFF243247),
                                RoundedCornerShape(100.dp)
                            )
                            .clickable {
                                if (!viewModel.isConnected) {
                                    android.widget.Toast.makeText(context, "CONNECT WORKSTATION FOR FILE SHARING", android.widget.Toast.LENGTH_SHORT).show()
                                } else {
                                    viewModel.fileSharingMode = 1 // Get (Download)
                                    viewModel.isFileSharingActive = true
                                    viewModel.fetchRemoteDirectory(null)
                                }
                            },
                        contentAlignment = Alignment.Center
                    ) {
                        Text(
                            text = "GET (DOWNLOAD)",
                            color = if (viewModel.isConnected) Color.White else Color(0xFF6B8B77),
                            fontSize = 12.sp,
                            fontWeight = FontWeight.ExtraBold,
                            letterSpacing = 1.5.sp
                        )
                    }
                }
            }

            // Discovery and Network Selection Dashboard
            item {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        text = "DISCOVERED WORKSTATIONS",
                        color = Color(0xFF94A3B8),
                        fontSize = 11.sp,
                        fontWeight = FontWeight.ExtraBold,
                        letterSpacing = 1.5.sp
                    )

                    // Glow button to scan LAN
                    Box(
                        modifier = Modifier
                            .clip(RoundedCornerShape(100.dp))
                            .background(
                                if (viewModel.isScanning) Color(0xFF0A1428) else Color(0xFF0E1624)
                            )
                            .border(
                                1.dp,
                                if (viewModel.isScanning) accentCyan else Color(0xFF243247),
                                RoundedCornerShape(100.dp)
                            )
                            .clickable(enabled = !viewModel.isScanning) {
                                viewModel.scanLocalSubnet()
                            }
                            .padding(horizontal = 12.dp, vertical = 6.dp)
                    ) {
                        Text(
                            text = if (viewModel.isScanning) "SCANNING..." else "SCAN WI-FI LAN",
                            color = if (viewModel.isScanning) accentCyan else Color(0xFFF5F7FA),
                            fontSize = 10.sp,
                            fontWeight = FontWeight.Bold,
                            letterSpacing = 1.sp
                        )
                    }
                }
            }

            // Discovered workstation list
            if (viewModel.discoveredWorkstations.isEmpty()) {
                item {
                    Box(
                        modifier = Modifier
                            .fillMaxWidth()
                            .height(100.dp)
                            .clip(RoundedCornerShape(topStart = 16.dp, bottomEnd = 16.dp))
                            .background(panelBg)
                            .border(1.dp, borderGradient, RoundedCornerShape(topStart = 16.dp, bottomEnd = 16.dp)),
                        contentAlignment = Alignment.Center
                    ) {
                        Canvas(modifier = Modifier.fillMaxSize()) {
                            val w = size.width
                            val h = size.height
                            val spacing = 20.dp.toPx()
                            val linesX = (w / spacing).toInt()
                            val linesY = (h / spacing).toInt()
                            for (i in 0..linesX) {
                                drawLine(
                                    color = Color(0xFF243247).copy(alpha = 0.3f),
                                    start = Offset(i * spacing, 0f),
                                    end = Offset(i * spacing, h),
                                    strokeWidth = 1f
                                )
                            }
                            for (i in 0..linesY) {
                                drawLine(
                                    color = Color(0xFF243247).copy(alpha = 0.3f),
                                    start = Offset(0f, i * spacing),
                                    end = Offset(w, i * spacing),
                                    strokeWidth = 1f
                                )
                            }
                        }
                        Text(
                            text = "NO WORKSTATIONS DETECTED",
                            color = Color(0xFF607086), // NYX_TEXT_MUTED
                            fontSize = 12.sp,
                            fontWeight = FontWeight.Bold,
                            textAlign = TextAlign.Center,
                            modifier = Modifier.padding(16.dp)
                        )
                    }
                }
            } else {
                items(viewModel.discoveredWorkstations) { workstation ->
                    WorkstationSelectCard(
                        workstation = workstation,
                        onClick = {
                            ipInput = workstation.ip
                            viewModel.connectToWorkstation(workstation.ip)
                        },
                        accentCyan = accentCyan,
                        panelBg = panelBg
                    )
                }
            }

            // DNS/MagicDNS Help Alert
            item {
                Spacer(modifier = Modifier.height(10.dp))
                Card(
                    shape = RoundedCornerShape(topStart = 16.dp, bottomEnd = 16.dp),
                    colors = CardDefaults.cardColors(containerColor = Color(0xFF0E1624)),
                    modifier = Modifier
                        .fillMaxWidth()
                        .border(1.dp, accentCyan.copy(alpha = 0.5f), RoundedCornerShape(topStart = 16.dp, bottomEnd = 16.dp))
                ) {
                    Column(
                        modifier = Modifier.padding(16.dp)
                    ) {
                        Text(
                            text = "TAILSCALE MAGICDNS TIP",
                            color = accentCyan,
                            fontSize = 10.sp,
                            fontWeight = FontWeight.ExtraBold,
                            letterSpacing = 1.sp
                        )
                        Spacer(modifier = Modifier.height(4.dp))
                        Text(
                            text = "If connected to Tailscale VPN on this device, you can enter your workstation's network hostname (e.g. nyxframe) instead of its IP address to resolve it automatically.",
                            color = Color(0xFF94A3B8),
                            fontSize = 11.sp,
                            fontWeight = FontWeight.Bold,
                            lineHeight = 15.sp
                        )
                    }
                }
            }
        }

        // Cyberpunk floating settings button in top-right corner - Declared at bottom of Box to float on top of LazyColumn
        Box(
            modifier = Modifier
                .align(Alignment.TopEnd)
                .padding(top = 24.dp, end = 24.dp)
                .width(76.dp)
                .height(34.dp)
                .clip(RoundedCornerShape(topStart = 8.dp, bottomEnd = 8.dp))
                .background(panelBg)
                .border(
                    1.dp,
                    accentCyan.copy(alpha = 0.4f),
                    RoundedCornerShape(topStart = 8.dp, bottomEnd = 8.dp)
                )
                .clickable { onNavigateToSettings() },
            contentAlignment = Alignment.Center
        ) {
            Text(
                text = "SETTINGS",
                color = Color.White,
                fontWeight = FontWeight.ExtraBold,
                fontSize = 9.sp,
                letterSpacing = 1.sp
            )
        }
    }

    // Connect navigator trigger
    var wasConnecting by remember { mutableStateOf(false) }
    LaunchedEffect(viewModel.isConnecting) {
        if (viewModel.isConnecting) {
            wasConnecting = true
        }
    }

    LaunchedEffect(viewModel.isConnected) {
        if (viewModel.isConnected && wasConnecting) {
            wasConnecting = false
            onNavigateToStream()
        }
    }

    if (viewModel.isFileSharingActive) {
        AlertDialog(
            onDismissRequest = {
                if (!viewModel.isTransferringFiles) {
                    viewModel.isFileSharingActive = false
                }
            },
            properties = androidx.compose.ui.window.DialogProperties(
                usePlatformDefaultWidth = false
            ),
            modifier = Modifier
                .fillMaxWidth(0.92f)
                .fillMaxHeight(0.85f)
                .clip(RoundedCornerShape(topStart = 16.dp, bottomEnd = 16.dp))
                .border(1.dp, accentCyan.copy(alpha = 0.5f), RoundedCornerShape(topStart = 16.dp, bottomEnd = 16.dp))
                .background(panelBg),
            title = {
                Column(modifier = Modifier.fillMaxWidth()) {
                    Text(
                        text = if (viewModel.fileSharingMode == 0) "PUSH DESTINATION" else "GET (DOWNLOAD) SYSTEM",
                        color = accentCyan,
                        fontSize = 16.sp,
                        fontWeight = FontWeight.ExtraBold,
                        letterSpacing = 2.sp
                    )
                    Spacer(modifier = Modifier.height(4.dp))
                    Text(
                        text = viewModel.currentRemotePath,
                        color = Color(0xFF94A3B8),
                        fontSize = 11.sp,
                        fontWeight = FontWeight.Bold,
                        modifier = Modifier.fillMaxWidth()
                    )
                }
            },
            text = {
                Box(modifier = Modifier.fillMaxSize()) {
                    if (viewModel.isTransferringFiles) {
                        Column(
                            modifier = Modifier
                                .fillMaxSize()
                                .padding(16.dp),
                            horizontalAlignment = Alignment.CenterHorizontally,
                            verticalArrangement = Arrangement.Center
                        ) {
                            CircularProgressIndicator(
                                color = accentCyan,
                                modifier = Modifier.size(48.dp)
                            )
                            Spacer(modifier = Modifier.height(16.dp))
                            Text(
                                text = "TRANSFERRING FILES...",
                                color = Color.White,
                                fontSize = 14.sp,
                                fontWeight = FontWeight.Bold,
                                letterSpacing = 1.5.sp
                            )
                            Spacer(modifier = Modifier.height(8.dp))
                            if (viewModel.fileTransferProgress > 0f) {
                                LinearProgressIndicator(
                                    progress = { viewModel.fileTransferProgress },
                                    color = accentCyan,
                                    trackColor = accentPurple,
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .height(8.dp)
                                        .clip(RoundedCornerShape(4.dp))
                                )
                                Spacer(modifier = Modifier.height(6.dp))
                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.SpaceBetween
                                ) {
                                    Text(
                                        text = "${(viewModel.fileTransferProgress * 100).toInt()}%",
                                        color = Color(0xFF94A3B8),
                                        fontSize = 12.sp,
                                        fontWeight = FontWeight.Bold
                                    )
                                    Text(
                                        text = viewModel.fileTransferProgressText,
                                        color = Color(0xFF94A3B8),
                                        fontSize = 12.sp,
                                        fontWeight = FontWeight.Bold
                                    )
                                }
                            } else {
                                LinearProgressIndicator(
                                    color = accentCyan,
                                    trackColor = accentPurple,
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .height(8.dp)
                                        .clip(RoundedCornerShape(4.dp))
                                )
                                Spacer(modifier = Modifier.height(6.dp))
                                Text(
                                    text = viewModel.fileTransferProgressText,
                                    color = Color(0xFF94A3B8),
                                    fontSize = 12.sp,
                                    fontWeight = FontWeight.Bold,
                                    textAlign = TextAlign.Center
                                )
                            }

                            if (viewModel.fileTransferSpeedText.isNotEmpty()) {
                                Spacer(modifier = Modifier.height(10.dp))
                                Text(
                                    text = viewModel.fileTransferSpeedText,
                                    color = accentCyan,
                                    fontSize = 13.sp,
                                    fontWeight = FontWeight.ExtraBold,
                                    textAlign = TextAlign.Center
                                )
                            }

                            Spacer(modifier = Modifier.height(24.dp))

                            // CANCEL TRANSFER BUTTON
                            Button(
                                onClick = {
                                    viewModel.cancelFileTransfer()
                                },
                                colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF1A0E2B)),
                                modifier = Modifier
                                    .width(180.dp)
                                    .height(42.dp)
                                    .border(1.dp, accentCyan.copy(alpha = 0.5f), RoundedCornerShape(10.dp)),
                                shape = RoundedCornerShape(10.dp)
                            ) {
                                Text(
                                    text = "CANCEL TRANSFER",
                                    color = Color.White,
                                    fontSize = 12.sp,
                                    fontWeight = FontWeight.Bold,
                                    letterSpacing = 1.sp
                                )
                            }
                        }
                    } else {
                        Column(modifier = Modifier.fillMaxSize()) {
                            LazyColumn(
                                modifier = Modifier
                                    .weight(1f)
                                    .fillMaxWidth(),
                                verticalArrangement = Arrangement.spacedBy(6.dp)
                            ) {
                                if (viewModel.remoteParentPath.isNotEmpty()) {
                                    item {
                                        Row(
                                            modifier = Modifier
                                                .fillMaxWidth()
                                                .clip(RoundedCornerShape(10.dp))
                                                .background(Color(0xFF1A1220))
                                                .clickable {
                                                    viewModel.fetchRemoteDirectory(viewModel.remoteParentPath)
                                                }
                                                .padding(12.dp),
                                            verticalAlignment = Alignment.CenterVertically
                                        ) {
                                            Text(
                                                text = "📁",
                                                fontSize = 18.sp,
                                                modifier = Modifier.padding(end = 10.dp)
                                            )
                                            Text(
                                                text = "../ (Go Up)",
                                                color = accentCyan,
                                                fontSize = 13.sp,
                                                fontWeight = FontWeight.Bold
                                            )
                                        }
                                    }
                                }

                                if (viewModel.remoteItems.isEmpty()) {
                                    item {
                                        Box(
                                            modifier = Modifier
                                                .fillMaxWidth()
                                                .padding(32.dp),
                                            contentAlignment = Alignment.Center
                                        ) {
                                            Text(
                                                text = "Empty Directory",
                                                color = Color(0xFF607086),
                                                fontSize = 12.sp,
                                                fontWeight = FontWeight.Bold
                                            )
                                        }
                                    }
                                } else {
                                    items(viewModel.remoteItems) { item ->
                                        val isSelected = viewModel.remoteSelectedItems.contains(item.path)
                                        Row(
                                            modifier = Modifier
                                                .fillMaxWidth()
                                                .clip(RoundedCornerShape(10.dp))
                                                .background(
                                                    if (isSelected) Color(0xFF33091B) else Color(0xFF0E1624)
                                                )
                                                .border(
                                                    1.dp,
                                                    if (isSelected) accentCyan.copy(alpha = 0.4f) else Color.Transparent,
                                                    RoundedCornerShape(10.dp)
                                                )
                                                .clickable {
                                                    if (item.isDir) {
                                                        viewModel.fetchRemoteDirectory(item.path)
                                                    } else if (viewModel.fileSharingMode == 1) {
                                                        if (isSelected) {
                                                            viewModel.remoteSelectedItems.remove(item.path)
                                                        } else {
                                                            viewModel.remoteSelectedItems.add(item.path)
                                                        }
                                                    }
                                                }
                                                .padding(12.dp),
                                            verticalAlignment = Alignment.CenterVertically
                                        ) {
                                            if (viewModel.fileSharingMode == 1) {
                                                Checkbox(
                                                    checked = isSelected,
                                                    onCheckedChange = { checked ->
                                                        if (checked == true) {
                                                            viewModel.remoteSelectedItems.add(item.path)
                                                        } else {
                                                            viewModel.remoteSelectedItems.remove(item.path)
                                                        }
                                                    },
                                                    colors = CheckboxDefaults.colors(
                                                        checkedColor = accentCyan,
                                                        uncheckedColor = Color(0xFF607086)
                                                    ),
                                                    modifier = Modifier.padding(end = 8.dp)
                                                )
                                            }

                                            Text(
                                                text = if (item.isDir) "📁" else "📄",
                                                fontSize = 18.sp,
                                                modifier = Modifier.padding(end = 10.dp)
                                            )

                                            Column(modifier = Modifier.weight(1f)) {
                                                Text(
                                                    text = item.name,
                                                    color = Color.White,
                                                    fontSize = 13.sp,
                                                    fontWeight = FontWeight.Bold,
                                                    maxLines = 1
                                                )
                                                if (!item.isDir) {
                                                    Spacer(modifier = Modifier.height(2.dp))
                                                    Text(
                                                        text = formatFileSize(item.size),
                                                        color = Color(0xFF94A3B8),
                                                        fontSize = 10.sp,
                                                        fontWeight = FontWeight.Medium
                                                    )
                                                }
                                            }

                                            if (item.isDir && viewModel.fileSharingMode == 1) {
                                                Box(
                                                    modifier = Modifier
                                                        .clip(RoundedCornerShape(6.dp))
                                                        .background(if (isSelected) accentCyan else Color(0xFF1D1426))
                                                        .clickable {
                                                            if (isSelected) {
                                                                viewModel.remoteSelectedItems.remove(item.path)
                                                            } else {
                                                                viewModel.remoteSelectedItems.add(item.path)
                                                            }
                                                        }
                                                        .padding(horizontal = 8.dp, vertical = 4.dp)
                                                ) {
                                                    Text(
                                                        text = if (isSelected) "SELECTED" else "SELECT DIR",
                                                        color = if (isSelected) Color.White else Color(0xFFF5F7FA),
                                                        fontSize = 9.sp,
                                                        fontWeight = FontWeight.ExtraBold
                                                    )
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
            confirmButton = {
                if (!viewModel.isTransferringFiles) {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(horizontal = 16.dp, vertical = 8.dp),
                        horizontalArrangement = Arrangement.spacedBy(12.dp)
                    ) {
                        Button(
                            onClick = {
                                viewModel.isFileSharingActive = false
                            },
                            colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF1D1426)),
                            contentPadding = PaddingValues(horizontal = 4.dp, vertical = 0.dp),
                            modifier = Modifier
                                .weight(1f)
                                .height(44.dp)
                                .border(1.dp, Color(0xFF243247), RoundedCornerShape(8.dp)),
                            shape = RoundedCornerShape(8.dp)
                        ) {
                            Text(
                                text = "CLOSE",
                                color = Color.White,
                                fontSize = 10.sp,
                                fontWeight = FontWeight.Bold,
                                maxLines = 1,
                                overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
                                textAlign = TextAlign.Center
                            )
                        }

                        Button(
                            onClick = {
                                newFolderName = ""
                                showNewFolderDialog = true
                            },
                            colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF243247)),
                            contentPadding = PaddingValues(horizontal = 4.dp, vertical = 0.dp),
                            modifier = Modifier
                                .weight(1f)
                                .height(44.dp)
                                .border(1.dp, accentCyan.copy(alpha = 0.4f), RoundedCornerShape(8.dp)),
                            shape = RoundedCornerShape(8.dp)
                        ) {
                            Text(
                                text = "NEW FOLDER",
                                color = Color.White,
                                fontSize = 10.sp,
                                fontWeight = FontWeight.Bold,
                                maxLines = 1,
                                overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
                                textAlign = TextAlign.Center
                            )
                        }

                        val actionText = if (viewModel.fileSharingMode == 0) "UPLOAD HERE" else "DOWNLOAD SELECTED (${viewModel.remoteSelectedItems.size})"
                        val isEnabled = if (viewModel.fileSharingMode == 0) true else viewModel.remoteSelectedItems.isNotEmpty()

                        Button(
                            onClick = {
                                if (viewModel.fileSharingMode == 0) {
                                    viewModel.uploadSelectedAndroidItems(
                                        uris = viewModel.pendingUploadUris,
                                        isFolder = viewModel.isPendingUploadDirectory,
                                        targetHostPath = viewModel.currentRemotePath,
                                        onSuccess = {
                                            android.widget.Toast.makeText(context, "UPLOAD SUCCESSFUL", android.widget.Toast.LENGTH_SHORT).show()
                                        },
                                        onError = { err ->
                                            android.widget.Toast.makeText(context, "UPLOAD FAILED: $err", android.widget.Toast.LENGTH_LONG).show()
                                        }
                                    )
                                } else {
                                    downloadFolderLauncher.launch(null)
                                }
                            },
                            enabled = isEnabled,
                            colors = ButtonDefaults.buttonColors(
                                containerColor = if (isEnabled) accentCyan else Color(0xFF151F31)
                            ),
                            contentPadding = PaddingValues(horizontal = 4.dp, vertical = 0.dp),
                            modifier = Modifier
                                .weight(1f)
                                .height(44.dp)
                                .border(
                                    1.dp,
                                    if (isEnabled) accentCyan.copy(alpha = 0.4f) else Color(0xFF243247).copy(alpha = 0.4f),
                                    RoundedCornerShape(8.dp)
                                ),
                            shape = RoundedCornerShape(8.dp)
                        ) {
                            Text(
                                text = actionText,
                                color = if (isEnabled) Color.White else Color(0xFF607086),
                                fontSize = 10.sp,
                                fontWeight = FontWeight.Bold,
                                maxLines = 1,
                                overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
                                textAlign = TextAlign.Center
                            )
                        }
                    }
                }
            }
        )

        if (showNewFolderDialog) {
            androidx.compose.ui.window.Dialog(
                onDismissRequest = { showNewFolderDialog = false },
                properties = androidx.compose.ui.window.DialogProperties(
                    usePlatformDefaultWidth = false
                )
            ) {
                Box(
                    modifier = Modifier
                        .fillMaxWidth(0.85f)
                        .clip(RoundedCornerShape(20.dp))
                        .border(1.dp, accentCyan.copy(alpha = 0.5f), RoundedCornerShape(20.dp))
                        .background(panelBg)
                        .padding(20.dp)
                ) {
                    Column(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalAlignment = Alignment.CenterHorizontally
                    ) {
                        Text(
                            text = "CREATE REMOTE FOLDER",
                            color = accentCyan,
                            fontSize = 15.sp,
                            fontWeight = FontWeight.ExtraBold,
                            letterSpacing = 1.5.sp
                        )
                        Spacer(modifier = Modifier.height(14.dp))
                        
                        Box(
                            modifier = Modifier
                                .fillMaxWidth()
                                .height(44.dp)
                                .clip(RoundedCornerShape(10.dp))
                                .background(Color(0xFF16101E))
                                .border(1.dp, accentCyan.copy(alpha = 0.25f), RoundedCornerShape(10.dp))
                                .padding(horizontal = 12.dp),
                            contentAlignment = Alignment.CenterStart
                        ) {
                            if (newFolderName.isEmpty()) {
                                Text(
                                    text = "Enter folder name...",
                                    color = Color(0xFF607086),
                                    fontSize = 13.sp,
                                    fontWeight = FontWeight.Bold
                                )
                            }
                            BasicTextField(
                                value = newFolderName,
                                onValueChange = { newFolderName = it },
                                textStyle = TextStyle(
                                    color = Color.White,
                                    fontSize = 13.sp,
                                    fontWeight = FontWeight.Bold
                                ),
                                singleLine = true,
                                cursorBrush = SolidColor(accentCyan),
                                modifier = Modifier.fillMaxWidth()
                            )
                        }
                        
                        Spacer(modifier = Modifier.height(20.dp))
                        
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(10.dp)
                        ) {
                            Button(
                                onClick = { showNewFolderDialog = false },
                                colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF1D1426)),
                                modifier = Modifier
                                    .weight(1f)
                                    .border(1.dp, Color(0xFF243247), RoundedCornerShape(8.dp)),
                                shape = RoundedCornerShape(8.dp)
                            ) {
                                Text(
                                    text = "CANCEL",
                                    color = Color.White,
                                    fontSize = 11.sp,
                                    fontWeight = FontWeight.Bold
                                )
                            }
                            
                            Button(
                                onClick = {
                                    val trimmed = newFolderName.trim()
                                    if (trimmed.isNotEmpty()) {
                                        viewModel.createRemoteDirectory(
                                            folderName = trimmed,
                                            onSuccess = {
                                                android.widget.Toast.makeText(context, "Folder Created Successfully", android.widget.Toast.LENGTH_SHORT).show()
                                                showNewFolderDialog = false
                                            },
                                            onError = { err ->
                                                android.widget.Toast.makeText(context, "Failed to Create Folder: $err", android.widget.Toast.LENGTH_LONG).show()
                                            }
                                        )
                                    }
                                },
                                enabled = newFolderName.trim().isNotEmpty(),
                                colors = ButtonDefaults.buttonColors(
                                    containerColor = if (newFolderName.trim().isNotEmpty()) accentCyan else Color(0xFF151F31)
                                ),
                                modifier = Modifier.weight(1f),
                                shape = RoundedCornerShape(8.dp)
                            ) {
                                Text(
                                    text = "CREATE",
                                    color = if (newFolderName.trim().isNotEmpty()) Color.White else Color(0xFF607086),
                                    fontSize = 11.sp,
                                    fontWeight = FontWeight.Bold
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}

fun formatFileSize(size: Long): String {
    if (size <= 0) return "0 B"
    val units = arrayOf("B", "KB", "MB", "GB", "TB")
    val digitGroups = (Math.log10(size.toDouble()) / Math.log10(1024.0)).toInt()
    return String.format("%.1f %s", size / Math.pow(1024.0, digitGroups.toDouble()), units[digitGroups])
}

@Composable
fun WorkstationSelectCard(
    workstation: DiscoveredWorkstation,
    onClick: () -> Unit,
    accentCyan: Color,
    panelBg: Color
) {
    val badgeColor = when (workstation.type) {
        "Wi-Fi LAN" -> Color(0xFF00E676)
        "Tailscale" -> accentCyan
        else -> Color(0xFF94A3B8) // Recent
    }

    Card(
        shape = RoundedCornerShape(topStart = 16.dp, bottomEnd = 16.dp),
        colors = CardDefaults.cardColors(containerColor = panelBg),
        modifier = Modifier
            .fillMaxWidth()
            .border(
                1.dp,
                Brush.horizontalGradient(listOf(badgeColor.copy(alpha = 0.3f), Color.Transparent)),
                RoundedCornerShape(topStart = 16.dp, bottomEnd = 16.dp)
            )
            .clickable { onClick() }
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.SpaceBetween
        ) {
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = workstation.name.uppercase(),
                    color = Color.White,
                    fontSize = 15.sp,
                    fontWeight = FontWeight.ExtraBold,
                    letterSpacing = 1.sp
                )
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    text = workstation.ip,
                    color = Color(0xFFF5F7FA),
                    fontSize = 13.sp,
                    fontWeight = FontWeight.Bold,
                    letterSpacing = 0.5.sp
                )
            }

            // Dynamic Badge indicators
            Box(
                modifier = Modifier
                    .clip(RoundedCornerShape(8.dp))
                    .background(badgeColor.copy(alpha = 0.15f))
                    .border(1.dp, badgeColor.copy(alpha = 0.5f), RoundedCornerShape(8.dp))
                    .padding(horizontal = 10.dp, vertical = 5.dp)
            ) {
                Text(
                    text = workstation.type.uppercase(),
                    color = badgeColor,
                    fontSize = 9.sp,
                    fontWeight = FontWeight.ExtraBold,
                    letterSpacing = 1.sp
                )
            }
        }
    }
}
