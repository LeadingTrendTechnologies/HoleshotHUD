#pragma once

extern "C" {

/******************************************************************************
 structures and functions to receive data from the simulated bike
 ******************************************************************************/

typedef struct
{
    char m_szRiderName[100];
    char m_szBikeID[100];
    char m_szBikeName[100];
    int m_iNumberOfGears;
    int m_iMaxRPM;
    int m_iLimiter;
    int m_iShiftRPM;
    float m_fEngineOptTemperature;
    float m_afEngineTemperatureAlarm[2];
    float m_fMaxFuel;
    float m_afSuspMaxTravel[2];
    float m_fSteerLock;
    char m_szCategory[100];
    char m_szTrackID[100];
    char m_szTrackName[100];
    float m_fTrackLength;
    int m_iType;
    char m_szServerName[64];
    int m_iServerType;
    char m_szGUID[100];
} SPluginsBikeEvent_t;

typedef struct
{
    int m_iSession;
    int m_iConditions;
    float m_fAirTemperature;
    char m_szSetupFileName[100];
} SPluginsBikeSession_t;

typedef struct
{
    int m_iRPM;
    float m_fEngineTemperature;
    float m_fWaterTemperature;
    int m_iGear;
    float m_fFuel;
    float m_fSpeedometer;
    float m_fPosX, m_fPosY, m_fPosZ;
    float m_fVelocityX, m_fVelocityY, m_fVelocityZ;
    float m_fAccelerationX, m_fAccelerationY, m_fAccelerationZ;
    float m_aafRot[3][3];
    float m_fYaw, m_fPitch, m_fRoll;
    float m_fYawVelocity, m_fPitchVelocity, m_fRollVelocity;
    float m_afSuspLength[2];
    float m_afSuspVelocity[2];
    int m_iCrashed;
    float m_fSteer;
    float m_fThrottle;
    float m_fFrontBrake;
    float m_fRearBrake;
    float m_fClutch;
    float m_afWheelSpeed[2];
    int m_aiWheelMaterial[2];
    float m_afBrakePressure[2];
    float m_fSteerTorque;
} SPluginsBikeData_t;

typedef struct
{
    int m_iLapNum;
    int m_iInvalid;
    int m_iLapTime;
    int m_iBest;
} SPluginsBikeLap_t;

typedef struct
{
    int m_iSplit;
    int m_iSplitTime;
    int m_iBestDiff;
} SPluginsBikeSplit_t;

/******************************************************************************
 structures and functions to draw
 ******************************************************************************/

typedef struct
{
    float m_aafPos[4][2];
    int m_iSprite;
    unsigned long m_ulColor;
} SPluginQuad_t;

typedef struct
{
    char m_szString[100];
    float m_afPos[2];
    int m_iFont;
    float m_fSize;
    int m_iJustify;
    unsigned long m_ulColor;
} SPluginString_t;

/******************************************************************************
 structures and functions to receive the track center line
 ******************************************************************************/

typedef struct
{
    int m_iType;
    float m_fLength;
    float m_fRadius;
    float m_fAngle;
    float m_afStart[2];
    float m_fHeight;
} SPluginsTrackSegment_t;

/******************************************************************************
 structures and functions to receive the race data
 ******************************************************************************/

typedef struct
{
    int m_iType;
    char m_szName[100];
    char m_szTrackName[100];
    float m_fTrackLength;
} SPluginsRaceEvent_t;

typedef struct
{
    int m_iRaceNum;
    char m_szName[100];
    char m_szBikeName[100];
    char m_szBikeShortName[100];
    char m_szCategory[100];
    int m_iUnactive;
    int m_iNumberOfGears;
    int m_iMaxRPM;
} SPluginsRaceAddEntry_t;

typedef struct
{
    int m_iRaceNum;
} SPluginsRaceRemoveEntry_t;

typedef struct
{
    int m_iSession;
    int m_iSessionState;
    int m_iSessionLength;
    int m_iSessionNumLaps;
    int m_iConditions;
    float m_fAirTemperature;
} SPluginsRaceSession_t;

typedef struct
{
    int m_iSession;
    int m_iSessionState;
    int m_iSessionLength;
} SPluginsRaceSessionState_t;

typedef struct
{
    int m_iSession;
    int m_iRaceNum;
    int m_iLapNum;
    int m_iInvalid;
    int m_iLapTime;
    int m_aiSplit[2];
    int m_iBest;
} SPluginsRaceLap_t;

typedef struct
{
    int m_iSession;
    int m_iRaceNum;
    int m_iLapNum;
    int m_iSplit;
    int m_iSplitTime;
} SPluginsRaceSplit_t;

typedef struct
{
    int m_iSession;
    int m_iRaceNum;
    int m_iTime;
} SPluginsRaceHoleshot_t;

typedef struct
{
    int m_iSession;
    int m_iRaceNum;
    int m_iCommunication;
    int m_iState;
    int m_iReason;
    int m_iOffence;
    int m_iLap;
    int m_iStart;
    int m_iType;
    int m_iTime;
} SPluginsRaceCommunication_t;

typedef struct
{
    int m_iSession;
    int m_iSessionState;
    int m_iSessionTime;
    int m_iNumEntries;
} SPluginsRaceClassification_t;

typedef struct
{
    int m_iRaceNum;
    int m_iState;
    int m_iBestLap;
    int m_iBestLapNum;
    int m_iNumLaps;
    int m_iGap;
    int m_iGapLaps;
    int m_iPenalty;
    int m_iPit;
} SPluginsRaceClassificationEntry_t;

typedef struct
{
    int m_iRaceNum;
    float m_fPosX, m_fPosY, m_fPosZ;
    float m_fYaw;
    float m_fTrackPos;
    int m_iCrashed;
} SPluginsRaceTrackPosition_t;

typedef struct
{
    int m_iRaceNum;
    int m_iActive;
    int m_iRPM;
    int m_iGear;
    float m_fSpeedometer;
    float m_fThrottle;
    float m_fFrontBrake;
    float m_fLean;
} SPluginsRaceVehicleData_t;

typedef struct
{
    int m_iRaceNum;
    char m_szName[100];
} SPluginsSpectateVehicle_t;

__declspec(dllexport) char* GetModID();
__declspec(dllexport) int GetModDataVersion();
__declspec(dllexport) int GetInterfaceVersion();
__declspec(dllexport) int Startup(char* _szSavePath);
__declspec(dllexport) void Shutdown();
__declspec(dllexport) void EventInit(void* _pData, int _iDataSize);
__declspec(dllexport) void EventDeinit();
__declspec(dllexport) void RunInit(void* _pData, int _iDataSize);
__declspec(dllexport) void RunDeinit();
__declspec(dllexport) void RunStart();
__declspec(dllexport) void RunStop();
__declspec(dllexport) void RunLap(void* _pData, int _iDataSize);
__declspec(dllexport) void RunSplit(void* _pData, int _iDataSize);
__declspec(dllexport) void RunTelemetry(void* _pData, int _iDataSize, float _fTime, float _fPos);
__declspec(dllexport) int DrawInit(int* _piNumSprites, char** _pszSpriteName, int* _piNumFonts, char** _pszFontName);
__declspec(dllexport) void Draw(int _iState, int* _piNumQuads, void** _ppQuad, int* _piNumString, void** _ppString);
__declspec(dllexport) void TrackCenterline(int _iNumSegments, SPluginsTrackSegment_t* _pasSegment, void* _pRaceData);
__declspec(dllexport) void RaceEvent(void* _pData, int _iDataSize);
__declspec(dllexport) void RaceDeinit();
__declspec(dllexport) void RaceAddEntry(void* _pData, int _iDataSize);
__declspec(dllexport) void RaceRemoveEntry(void* _pData, int _iDataSize);
__declspec(dllexport) void RaceSession(void* _pData, int _iDataSize);
__declspec(dllexport) void RaceSessionState(void* _pData, int _iDataSize);
__declspec(dllexport) void RaceLap(void* _pData, int _iDataSize);
__declspec(dllexport) void RaceSplit(void* _pData, int _iDataSize);
__declspec(dllexport) void RaceHoleshot(void* _pData, int _iDataSize);
__declspec(dllexport) void RaceCommunication(void* _pData, int _iDataSize);
__declspec(dllexport) void RaceClassification(void* _pData, int _iDataSize, void* _pArray, int _iElemSize);
__declspec(dllexport) void RaceTrackPosition(int _iNumVehicles, void* _pArray, int _iElemSize);
__declspec(dllexport) void RaceVehicleData(void* _pData, int _iDataSize);
__declspec(dllexport) int SpectateVehicles(int _iNumVehicles, void* _pVehicleData, int _iCurSelection, int* _piSelect);
__declspec(dllexport) int SpectateCameras(int _iNumCameras, void* _pCameraData, int _iCurSelection, int* _piSelect);

}
